use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;

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
}

fn env_or_dev(name: &str, build_default: &str, dev_default: &str) -> String {
    // A distribuição de demonstração funciona com o Docker local sem exigir que
    // o usuário saiba injetar variáveis no executável. Produção sobrescreve todos
    // estes valores no instalador/configuração gerenciada.
    std::env::var(name).unwrap_or_else(|_| {
        if !build_default.is_empty() {
            build_default.into()
        } else if cfg!(debug_assertions) {
            dev_default.into()
        } else {
            String::new()
        }
    })
}

fn api_url() -> String {
    env_or_dev(
        "COMMERCECTRL_API_URL",
        option_env!("COMMERCECTRL_API_URL").unwrap_or(""),
        "http://127.0.0.1:8088",
    )
}

fn tenant_id() -> String {
    env_or_dev(
        "COMMERCECTRL_TENANT_ID",
        option_env!("COMMERCECTRL_TENANT_ID").unwrap_or(""),
        "00000000-0000-0000-0000-000000000001",
    )
}

fn unit_id() -> String {
    env_or_dev(
        "COMMERCECTRL_UNIT_ID",
        option_env!("COMMERCECTRL_UNIT_ID").unwrap_or(""),
        "00000000-0000-0000-0000-000000000101",
    )
}

fn terminal_id() -> String {
    env_or_dev(
        "COMMERCECTRL_TERMINAL_ID",
        option_env!("COMMERCECTRL_TERMINAL_ID").unwrap_or(""),
        "00000000-0000-0000-0000-000000001001",
    )
}

fn terminal_key() -> String {
    env_or_dev(
        "COMMERCECTRL_TERMINAL_KEY",
        option_env!("COMMERCECTRL_TERMINAL_KEY").unwrap_or(""),
        "commercectrl-dev-key",
    )
}

fn authenticated(request: ureq::Request) -> ureq::Request {
    request
        .set("X-Tenant-Id", &tenant_id())
        .set("X-Unit-Id", &unit_id())
        .set("X-Terminal-Id", &terminal_id())
        .set("X-Terminal-Key", &terminal_key())
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
        !api_url().is_empty()
            && !tenant_id().is_empty()
            && !unit_id().is_empty()
            && !terminal_id().is_empty()
            && !terminal_key().is_empty()
    }

    pub fn sync_pending(&self) -> rusqlite::Result<usize> {
        if !self.is_configured() {
            return Ok(0);
        }
        let rows = self.db.list_pending_outbox(100)?;
        let mut synced = 0usize;
        for row in rows {
            match push_to_api(&row) {
                Ok(true) => {
                    self.db.mark_outbox_sent(row.id)?;
                    synced += 1;
                }
                Ok(false) | Err(_) => {
                    self.db.mark_outbox_failed(row.id, "falha de rede")?;
                }
            }
        }
        Ok(synced)
    }
}

fn push_to_api(row: &OutboxRow) -> Result<bool, String> {
    if api_url().is_empty() {
        return Err("sincronização não configurada".into());
    }
    let endpoint = format!("{}/api/v1/sync/outbox", api_url());
    let body = serde_json::json!({
        "uuid": row.uuid,
        "entity": row.entity,
        "operation": row.operation,
        "payload": row.payload,
        "created_at": row.created_at,
    })
    .to_string();

    let resp = authenticated(ureq::post(&endpoint))
        .timeout(Duration::from_secs(5))
        .set("Content-Type", "application/json")
        .send_string(&body)
        .map_err(|e| format!("{e}"))?;
    Ok((200..300).contains(&resp.status()))
}

fn pull_products(db: &Db) -> Result<usize, String> {
    if api_url().is_empty() {
        return Ok(0);
    }
    let resp = authenticated(ureq::get(&format!("{}/api/v1/sync/products", api_url())))
        .timeout(Duration::from_secs(5))
        .call()
        .map_err(|e| format!("{e}"))?;
    if !(200..300).contains(&resp.status()) {
        return Ok(0);
    }
    let body = resp.into_string().map_err(|e| format!("{e}"))?;
    let products: Vec<RemoteProduct> = serde_json::from_str(&body).map_err(|e| format!("{e}"))?;
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
    }
    Ok(products.len())
}

pub fn worker(state: Arc<AppState>, _api_url: String) {
    loop {
        std::thread::sleep(Duration::from_secs(5));

        if !state.outbox.is_configured() {
            state.set_online(false);
            state.refresh_pending();
            continue;
        }

        let mut online = false;
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
            Err(e) => log::warn!("Outbox: erro {e}"),
        }

        match pull_products(&state.db) {
            Ok(n) if n > 0 => {
                log::info!("Produtos sincronizados do servidor: {n}");
                online = true;
                state.publish(
                    serde_json::json!({"event": "products_synced", "count": n}).to_string(),
                );
            }
            Ok(_) => online = true,
            Err(e) => log::debug!("Delta-sync de produtos: {e}"),
        }

        state.set_online(online);
        state.refresh_pending();
    }
}
