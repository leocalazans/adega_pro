use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::db::{Db, OutboxRow};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
struct RemoteProduct {
    ean: String,
    part_number: String,
    description: String,
    brand: Option<String>,
    price_brl_cents: i64,
    stock_qty: f64,
    min_stock: f64,
    active: bool,
    image_url: Option<String>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
struct RemoteUser {
    id: String,
    email: String,
    display_name: Option<String>,
    role: String,
    password_hash: String,
    must_change_password: bool,
    active: bool,
    updated_at: chrono::DateTime<chrono::Utc>,
}

const TENANT_ID_SETTING: &str = "activation.tenant_id";
const UNIT_ID_SETTING: &str = "activation.unit_id";
const TERMINAL_ID_SETTING: &str = "activation.terminal_id";
const TERMINAL_KEY_SETTING: &str = "activation.terminal_key";
const API_URL_SETTING: &str = "activation.api_url";
const OWNER_KEY_SETTING: &str = "activation.owner_key";
const TENANT_SLUG_SETTING: &str = "activation.tenant_slug";
const TENANT_NAME_SETTING: &str = "activation.tenant_name";
const PRODUCTS_CURSOR_SETTING: &str = "sync.products_cursor";
const USERS_CURSOR_SETTING: &str = "sync.users_cursor";
/// Janela de sobreposição no delta-sync. Duas gravações podem compartilhar o
/// mesmo instante e um commit atrasado pode ficar com timestamp anterior ao
/// cursor; repetir alguns segundos é inofensivo porque os upserts são
/// idempotentes.
const CURSOR_OVERLAP_MILLIS: i64 = 5_000;

#[derive(Debug, Clone, Serialize)]
pub struct ActivationStatus {
    pub activated: bool,
    pub api_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TerminalCredentials {
    pub tenant_id: String,
    pub unit_id: String,
    pub terminal_id: String,
    pub terminal_key: String,
    pub api_url: String,
}

pub fn api_url(db: &Db) -> String {
    db.get_setting(API_URL_SETTING)
        .ok()
        .flatten()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            option_env!("COMMERCECTRL_API_URL")
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| {
            if cfg!(debug_assertions) {
                "http://127.0.0.1:8088".into()
            } else {
                String::new()
            }
        })
}

pub fn credentials(db: &Db) -> Option<TerminalCredentials> {
    let get = |key| {
        db.get_setting(key)
            .ok()
            .flatten()
            .filter(|v| !v.trim().is_empty())
    };
    Some(TerminalCredentials {
        tenant_id: get(TENANT_ID_SETTING)?,
        unit_id: get(UNIT_ID_SETTING)?,
        terminal_id: get(TERMINAL_ID_SETTING)?,
        terminal_key: get(TERMINAL_KEY_SETTING)?,
        api_url: api_url(db),
    })
    .filter(|v| !v.api_url.is_empty())
}

pub fn activation_status(db: &Db) -> ActivationStatus {
    ActivationStatus {
        activated: credentials(db).is_some(),
        api_url: (!api_url(db).is_empty()).then(|| api_url(db)),
    }
}

/// Credencial do proprietário: prioriza a devolvida na ativação do terminal,
/// depois a variável de ambiente usada no provisionamento antigo.
pub fn owner_credentials(db: &Db) -> Option<(String, String)> {
    let tenant_id = db
        .get_setting(TENANT_ID_SETTING)
        .ok()
        .flatten()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            std::env::var("COMMERCECTRL_TENANT_ID")
                .ok()
                .filter(|v| !v.is_empty())
        })
        .or_else(|| {
            cfg!(debug_assertions).then(|| "00000000-0000-0000-0000-000000000001".to_string())
        })?;
    let owner_key = db
        .get_setting(OWNER_KEY_SETTING)
        .ok()
        .flatten()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            std::env::var("COMMERCECTRL_OWNER_KEY")
                .ok()
                .filter(|v| !v.is_empty())
        })
        .or_else(|| cfg!(debug_assertions).then(|| "commercectrl-owner-dev-key".to_string()))?;
    Some((tenant_id, owner_key))
}

pub fn tenant_slug(db: &Db) -> Option<String> {
    db.get_setting(TENANT_SLUG_SETTING)
        .ok()
        .flatten()
        .filter(|v| !v.trim().is_empty())
}

pub fn persist_activation_metadata(
    db: &Db,
    slug: &str,
    name: &str,
    owner_key: &str,
) -> Result<(), String> {
    for (key, value) in [
        (TENANT_SLUG_SETTING, slug),
        (TENANT_NAME_SETTING, name),
        (OWNER_KEY_SETTING, owner_key),
    ] {
        db.set_setting(key, value).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn persist_credentials(db: &Db, credentials: &TerminalCredentials) -> Result<(), String> {
    for (key, value) in [
        (TENANT_ID_SETTING, &credentials.tenant_id),
        (UNIT_ID_SETTING, &credentials.unit_id),
        (TERMINAL_ID_SETTING, &credentials.terminal_id),
        (TERMINAL_KEY_SETTING, &credentials.terminal_key),
        (API_URL_SETTING, &credentials.api_url),
    ] {
        db.set_setting(key, value).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn authenticated(request: ureq::Request, credentials: &TerminalCredentials) -> ureq::Request {
    request
        .set("X-Tenant-Id", &credentials.tenant_id)
        .set("X-Unit-Id", &credentials.unit_id)
        .set("X-Terminal-Id", &credentials.terminal_id)
        .set("X-Terminal-Key", &credentials.terminal_key)
}

pub struct Outbox {
    db: Arc<Db>,
}

impl Outbox {
    pub fn new(db: Arc<Db>) -> Self {
        Self { db }
    }

    pub fn pending(&self) -> rusqlite::Result<Vec<OutboxRow>> {
        self.db.list_pending_outbox(100)
    }

    pub fn is_configured(&self) -> bool {
        credentials(&self.db).is_some()
    }

    pub fn sync_pending(&self) -> rusqlite::Result<usize> {
        if !self.is_configured() {
            return Ok(0);
        }
        let rows = self.db.list_pending_outbox(100)?;
        let mut synced = 0usize;
        for row in rows {
            match push_to_api(&self.db, &row) {
                Ok(true) => {
                    self.db.mark_outbox_sent(row.id)?;
                    synced += 1;
                }
                Ok(false) => {
                    // Resposta não-2xx: vale seguir para os próximos itens,
                    // pois um payload inválido não impede os demais.
                    self.db.mark_outbox_failed(row.id, "resposta não-2xx")?;
                }
                Err(error) => {
                    // Falha de transporte: o restante também vai falhar e
                    // cada tentativa custa um timeout. Encerra o ciclo.
                    self.db.mark_outbox_failed(row.id, &error)?;
                    break;
                }
            }
        }
        Ok(synced)
    }
}

fn push_to_api(db: &Db, row: &OutboxRow) -> Result<bool, String> {
    let credentials = credentials(db).ok_or("terminal não ativado")?;
    let endpoint = format!("{}/api/v1/sync/outbox", credentials.api_url);
    let body = serde_json::json!({
        "uuid": row.uuid,
        "entity": row.entity,
        "operation": row.operation,
        "payload": row.payload,
        "created_at": row.created_at,
    })
    .to_string();

    let resp = authenticated(ureq::post(&endpoint), &credentials)
        .timeout(Duration::from_secs(5))
        .set("Content-Type", "application/json")
        .send_string(&body)
        .map_err(|e| format!("{e}"))?;
    Ok((200..300).contains(&resp.status()))
}

fn read_cursor(db: &Db, key: &str) -> Option<i64> {
    db.get_setting(key)
        .ok()
        .flatten()
        .and_then(|value| value.trim().parse::<i64>().ok())
        .filter(|value| *value > 0)
}

fn store_cursor(db: &Db, key: &str, value: i64) -> Result<(), String> {
    if value > 0 {
        db.set_setting(key, &value.to_string())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn pull_products(db: &Db, full: bool) -> Result<usize, String> {
    let credentials = credentials(db).ok_or("terminal não ativado")?;
    let mut endpoint = format!("{}/api/v1/sync/products", credentials.api_url);
    let stored = read_cursor(db, PRODUCTS_CURSOR_SETTING);
    let since = (!full).then_some(stored).flatten();
    if let Some(cursor) = since {
        endpoint.push_str(&format!(
            "?since={}",
            cursor.saturating_sub(CURSOR_OVERLAP_MILLIS)
        ));
    }
    let resp = authenticated(ureq::get(&endpoint), &credentials)
        .timeout(Duration::from_secs(5))
        .call()
        .map_err(|e| format!("{e}"))?;
    if !(200..300).contains(&resp.status()) {
        return Ok(0);
    }
    let body = resp.into_string().map_err(|e| format!("{e}"))?;
    let products: Vec<RemoteProduct> = serde_json::from_str(&body).map_err(|e| format!("{e}"))?;
    let mut newest = stored.unwrap_or(0);
    for p in &products {
        db.upsert_product(
            &p.ean,
            &p.part_number,
            &p.description,
            p.brand.as_deref(),
            p.price_brl_cents,
            p.stock_qty,
        )
        .map_err(|e| format!("{e}"))?;
        db.set_product_min_stock(&p.ean, p.min_stock)
            .map_err(|e| format!("{e}"))?;
        db.set_product_active(&p.ean, p.active)
            .map_err(|e| format!("{e}"))?;
        db.set_product_image(&p.ean, p.image_url.as_deref())
            .map_err(|e| format!("{e}"))?;
        if let Some(updated_at) = p.updated_at {
            newest = newest.max(updated_at.timestamp_millis());
        }
    }
    store_cursor(db, PRODUCTS_CURSOR_SETTING, newest)?;
    Ok(products.len())
}

pub fn pull_users(db: &Db, full: bool) -> Result<usize, String> {
    let credentials = credentials(db).ok_or("terminal não ativado")?;
    let mut endpoint = format!("{}/api/v1/sync/users", credentials.api_url);
    let stored = read_cursor(db, USERS_CURSOR_SETTING);
    let since = (!full).then_some(stored).flatten();
    if let Some(cursor) = since {
        endpoint.push_str(&format!(
            "?since={}",
            cursor.saturating_sub(CURSOR_OVERLAP_MILLIS)
        ));
    }
    let resp = authenticated(ureq::get(&endpoint), &credentials)
        .timeout(Duration::from_secs(5))
        .call()
        .map_err(|e| format!("{e}"))?;
    if !(200..300).contains(&resp.status()) {
        return Ok(0);
    }
    let body = resp.into_string().map_err(|e| format!("{e}"))?;
    let users: Vec<RemoteUser> = serde_json::from_str(&body).map_err(|e| format!("{e}"))?;
    let mut newest = stored.unwrap_or(0);
    for user in &users {
        let role = match user.role.as_str() {
            "owner" | "admin" | "superadmin" => "admin",
            "manager" => "manager",
            "stock" => "stock",
            "finance" => "finance",
            "viewer" => "viewer",
            _ => "cashier",
        };
        let display_name = user
            .display_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(&user.email);
        db.upsert_cloud_user(
            &user.id,
            &user.email,
            display_name,
            role,
            &user.password_hash,
            user.must_change_password,
            user.active,
            user.updated_at.timestamp(),
        )
        .map_err(|e| format!("{e}"))?;
        newest = newest.max(user.updated_at.timestamp_millis());
    }
    if !users.is_empty() {
        db.retire_local_admin().map_err(|e| format!("{e}"))?;
    }
    store_cursor(db, USERS_CURSOR_SETTING, newest)?;
    Ok(users.len())
}

/// Renova a licença do terminal. Um 402 do servidor significa assinatura
/// vencida/suspensa: marca o início da carência de 7 dias no PDV.
pub fn renew_license(db: &Db) -> Result<crate::license::LicenseStatus, String> {
    let credentials = credentials(db).ok_or("terminal não ativado")?;
    let installation_id = crate::license::installation_id(db)?;
    let body = serde_json::json!({"installation_id": installation_id}).to_string();
    let result = authenticated(
        ureq::post(&format!("{}/api/v1/licenses/renew", credentials.api_url)),
        &credentials,
    )
    .set("Content-Type", "application/json")
    .timeout(Duration::from_secs(8))
    .send_string(&body);
    let response = match result {
        Ok(response) => response,
        Err(ureq::Error::Status(402, _)) => {
            crate::license::mark_payment_blocked(db)?;
            return Err("assinatura suspensa: renove para voltar a vender".into());
        }
        Err(error) => return Err(format!("renovação de licença indisponível: {error}")),
    };
    if !(200..300).contains(&response.status()) {
        return Err(format!(
            "renovação de licença recusada (HTTP {})",
            response.status()
        ));
    }
    let raw = response.into_string().map_err(|e| e.to_string())?;
    crate::license::import_token(db, &raw, Some("online"))
}

pub fn worker(state: Arc<AppState>, _api_url: String) {
    // Ciclo periódico de 3 minutos; vendas e cadastros acordam o worker na hora.
    const PERIOD: Duration = Duration::from_secs(180);
    const DRAIN: Duration = Duration::from_secs(10);
    const FULL_PULL_SECS: i64 = 86_400;
    const LICENSE_SECS: i64 = 600;

    let mut first = true;
    let mut failure_streak: u32 = 0;
    let mut last_full_pull: i64 = 0;
    let mut last_license_check: i64 = 0;

    loop {
        if !first {
            let pending = state.db.count_pending_outbox().unwrap_or(0);
            let wait = if failure_streak > 0 {
                // 5, 10, 20, 40, 80, 160, 180 segundos.
                let seconds = 5u64
                    .saturating_mul(1u64 << (failure_streak - 1).min(5))
                    .min(PERIOD.as_secs());
                Duration::from_secs(seconds)
            } else if pending > 0 {
                DRAIN
            } else {
                PERIOD
            };
            state.wait_for_wake(wait);
        }
        first = false;

        if !state.outbox.is_configured() {
            state.set_online(false);
            state.refresh_pending();
            continue;
        }

        let now = chrono::Utc::now().timestamp();
        let full = now - last_full_pull >= FULL_PULL_SECS;
        let mut online = false;
        let mut pulls_ok = true;

        match state.outbox.sync_pending() {
            Ok(count) if count > 0 => {
                log::info!("Outbox: {count} itens sincronizados");
                online = true;
                state.publish(
                    serde_json::json!({"event": "outbox_synced", "count": count}).to_string(),
                );
            }
            // Sem itens na outbox nenhuma chamada de rede foi feita; o pull abaixo
            // é quem confirma que o backend está realmente acessível.
            Ok(_) => {}
            Err(e) => {
                log::warn!("Outbox: erro {e}");
                pulls_ok = false;
            }
        }

        match pull_products(&state.db, full) {
            Ok(n) => {
                online = true;
                if n > 0 {
                    log::info!("Produtos sincronizados do servidor: {n}");
                    state.publish(
                        serde_json::json!({"event": "products_synced", "count": n}).to_string(),
                    );
                }
            }
            Err(e) => {
                pulls_ok = false;
                log::debug!("Delta-sync de produtos: {e}");
            }
        }

        match pull_users(&state.db, full) {
            Ok(n) => {
                online = true;
                if n > 0 {
                    log::info!("Usuários sincronizados do servidor: {n}");
                    state.publish(
                        serde_json::json!({"event": "users_synced", "count": n}).to_string(),
                    );
                }
            }
            Err(e) => {
                pulls_ok = false;
                log::debug!("Delta-sync de usuários: {e}");
            }
        }

        if full && online && pulls_ok {
            last_full_pull = now;
        }

        // A cada ~10 minutos confirma a assinatura com a nuvem: cortesia
        // estendida chega ao PDV e uma suspensão inicia a carência de 7 dias.
        if now - last_license_check >= LICENSE_SECS {
            last_license_check = now;
            match renew_license(&state.db) {
                Ok(status) => {
                    online = true;
                    state.publish(
                        serde_json::json!({"event": "license_synced", "mode": status.mode})
                            .to_string(),
                    );
                }
                Err(e) => log::debug!("Renovação de licença: {e}"),
            }
        }

        state.set_online(online);
        state.refresh_pending();

        if online {
            failure_streak = 0;
        } else {
            failure_streak = (failure_streak + 1).min(6);
        }
    }
}
