#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod financial;
mod fiscal;
mod license;
mod payments;
mod printer;
mod server;
mod state;
mod sync;
mod win32;

use std::sync::Arc;

use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tauri::Manager;

use crate::db::{
    CashCounted, Customer, DashboardSummary, Employee, Expense, PaymentIn, PaymentTotals, Product,
    Promotion, SaleItemIn, SaleOut, SalesReportRow, Supplier, TimeEntry,
};
use crate::printer::PrintJob;
use crate::state::AppState;
use crate::state::UserSession;

#[derive(Debug, Clone, Serialize)]
struct BarcodePayload {
    code: String,
    at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UnitSalesReport {
    unit_id: String,
    unit_name: String,
    sales_count: i64,
    total_brl_cents: i64,
    items_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OwnerUnitReport {
    unit_id: String,
    unit_name: String,
    unit_code: String,
    sales_count: i64,
    total_brl_cents: i64,
    items_count: i64,
    stock_skus: i64,
    low_stock_skus: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OwnerTopProductReport {
    ean: String,
    description: String,
    qty_sold: f64,
    revenue_brl_cents: i64,
    units_sold_in: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OwnerOverview {
    total_brl_cents: i64,
    sales_count: i64,
    units: Vec<OwnerUnitReport>,
    top_products: Vec<OwnerTopProductReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TenantBranding {
    display_name: String,
    logo_data_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct LocalDisplayLink {
    url: String,
    port: u16,
    note: String,
}

#[derive(Debug, Clone, Serialize)]
struct KioskAdminStatus {
    configured: bool,
}

const KIOSK_ADMIN_PIN_HASH_KEY: &str = "kiosk_admin_pin_hash";

#[derive(Debug, Clone, Serialize)]
struct LoginResult {
    session: UserSession,
    must_change_password: bool,
}

fn permissions_for(role: &str) -> Vec<String> {
    let values: &[&str] = match role {
        "superadmin" | "admin" => &["*"],
        "manager" => &[
            "pos",
            "cash",
            "catalog",
            "inventory",
            "customers",
            "employees",
            "time",
            "finance",
            "reports",
            "promotions",
            "display",
            "labels",
            "settings",
        ],
        "cashier" => &["pos", "cash", "customers", "time"],
        "stock" => &["catalog", "inventory", "suppliers", "labels", "time"],
        "finance" => &["finance", "reports", "cash", "time"],
        "viewer" => &["reports"],
        _ => &["time"],
    };
    values.iter().map(|v| (*v).to_string()).collect()
}

fn password_hash(value: &str) -> Result<String, String> {
    if value.chars().count() < 6 {
        return Err("A senha deve ter pelo menos 6 caracteres".into());
    }
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(value.as_bytes(), &salt)
        .map(|v| v.to_string())
        .map_err(|_| "Não foi possível proteger a senha".into())
}

#[tauri::command]
fn login_employee(
    state: tauri::State<'_, Arc<AppState>>,
    username: String,
    password: String,
) -> Result<LoginResult, String> {
    let user = state
        .db
        .auth_user(username.trim())
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Usuário ou senha inválidos".to_string())?;
    if let Some(stored) = &user.password_hash {
        let parsed = PasswordHash::new(stored).map_err(|_| "Credencial local inválida")?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .map_err(|_| "Usuário ou senha inválidos")?;
    } else if !(user.username.eq_ignore_ascii_case("admin") && password == "123456") {
        return Err("Usuário ou senha inválidos".into());
    } else {
        state
            .db
            .set_user_password(user.id, &password_hash(&password)?, true)
            .map_err(|e| e.to_string())?;
    }
    let session = UserSession {
        user_id: user.id,
        employee_id: user.employee_id,
        display_name: user.display_name,
        username: user.username,
        role: user.role.clone(),
        permissions: permissions_for(&user.role),
    };
    state.set_user(session.clone());
    Ok(LoginResult {
        session,
        must_change_password: user.must_change_password,
    })
}

#[tauri::command]
fn current_session(state: tauri::State<'_, Arc<AppState>>) -> Option<UserSession> {
    state.user()
}

#[tauri::command]
fn logout_employee(state: tauri::State<'_, Arc<AppState>>) {
    state.clear_user();
}

#[tauri::command]
fn change_own_password(
    state: tauri::State<'_, Arc<AppState>>,
    current_password: String,
    new_password: String,
) -> Result<(), String> {
    let session = state
        .user()
        .ok_or_else(|| "Faça login novamente".to_string())?;
    let user = state
        .db
        .auth_user(&session.username)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Usuário não encontrado".to_string())?;
    let stored = user
        .password_hash
        .ok_or_else(|| "Credencial inválida".to_string())?;
    let parsed = PasswordHash::new(&stored).map_err(|_| "Credencial inválida")?;
    Argon2::default()
        .verify_password(current_password.as_bytes(), &parsed)
        .map_err(|_| "Senha atual inválida")?;
    state
        .db
        .set_user_password(user.id, &password_hash(&new_password)?, false)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_terminal_backup(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    state.require("settings")?;
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("backups");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let target = dir.join(format!(
        "commercectrl-terminal-{}.db",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    ));
    state
        .db
        .backup_to(&target)
        .map_err(|e| format!("Backup: {e}"))?;
    crate::db::Db::validate_backup(&target)?;
    Ok(target.to_string_lossy().into_owned())
}

#[tauri::command]
fn schedule_terminal_restore(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
) -> Result<(), String> {
    state.require("settings")?;
    let source = std::path::PathBuf::from(path);
    crate::db::Db::validate_backup(&source)?;
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let staged = app_dir.join("restore-staged.db");
    std::fs::copy(&source, &staged)
        .map_err(|e| format!("Não foi possível preparar a restauração: {e}"))?;
    std::fs::write(
        app_dir.join("restore.pending"),
        staged.to_string_lossy().as_bytes(),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn validate_kiosk_pin(pin: &str) -> Result<(), String> {
    if !(6..=128).contains(&pin.chars().count()) {
        return Err("A senha administrativa deve ter entre 6 e 128 caracteres".into());
    }
    Ok(())
}

#[tauri::command]
fn kiosk_admin_status(state: tauri::State<'_, Arc<AppState>>) -> Result<KioskAdminStatus, String> {
    Ok(KioskAdminStatus {
        configured: state
            .db
            .get_setting(KIOSK_ADMIN_PIN_HASH_KEY)
            .map_err(|e| e.to_string())?
            .is_some(),
    })
}

/// The first administrator PIN must be configured during the controlled installation step.
/// Once persisted, this command intentionally refuses replacements; changing it belongs to
/// the future authenticated admin account flow, not an unauthenticated kiosk screen.
#[tauri::command]
fn configure_kiosk_admin_pin(
    pin: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    validate_kiosk_pin(&pin)?;
    if state
        .db
        .get_setting(KIOSK_ADMIN_PIN_HASH_KEY)
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Err("A senha administrativa já foi configurada".into());
    }
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(pin.as_bytes(), &salt)
        .map_err(|_| "Não foi possível proteger a senha administrativa")?
        .to_string();
    state
        .db
        .set_setting(KIOSK_ADMIN_PIN_HASH_KEY, &hash)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn exit_kiosk_as_admin(
    pin: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let stored = state
        .db
        .get_setting(KIOSK_ADMIN_PIN_HASH_KEY)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "A senha administrativa ainda não foi configurada".to_string())?;
    let parsed =
        PasswordHash::new(&stored).map_err(|_| "Configuração de administrador inválida")?;
    Argon2::default()
        .verify_password(pin.as_bytes(), &parsed)
        .map_err(|_| "Senha administrativa inválida")?;
    state.authorize_kiosk_exit();
    app.exit(0);
    Ok(())
}

#[tauri::command]
fn verify_admin_pin(pin: String, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    let stored = state
        .db
        .get_setting(KIOSK_ADMIN_PIN_HASH_KEY)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "A senha administrativa ainda não foi configurada".to_string())?;
    let parsed =
        PasswordHash::new(&stored).map_err(|_| "Configuração de administrador inválida")?;
    Argon2::default()
        .verify_password(pin.as_bytes(), &parsed)
        .map_err(|_| "Senha administrativa inválida".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CloudPaymentIntent {
    id: String,
    provider: String,
    provider_id: Option<String>,
    external_reference: String,
    amount_brl_cents: i64,
    status: String,
    qr_payload: Option<String>,
    expires_at: Option<String>,
}

fn cloud_request(request: ureq::Request) -> ureq::Request {
    let value = |name: &str, dev: &str| {
        std::env::var(name).unwrap_or_else(|_| {
            if cfg!(debug_assertions) {
                dev.into()
            } else {
                String::new()
            }
        })
    };
    request
        .set(
            "X-Tenant-Id",
            &value(
                "COMMERCECTRL_TENANT_ID",
                "00000000-0000-0000-0000-000000000001",
            ),
        )
        .set(
            "X-Unit-Id",
            &value(
                "COMMERCECTRL_UNIT_ID",
                "00000000-0000-0000-0000-000000000101",
            ),
        )
        .set(
            "X-Terminal-Id",
            &value(
                "COMMERCECTRL_TERMINAL_ID",
                "00000000-0000-0000-0000-000000001001",
            ),
        )
        .set(
            "X-Terminal-Key",
            &value("COMMERCECTRL_TERMINAL_KEY", "commercectrl-dev-key"),
        )
        .timeout(std::time::Duration::from_secs(8))
}

fn cloud_api_url() -> String {
    // A demo instalada conversa com o backend Docker local. Em produção o
    // instalador fornece a URL Render/Supabase por variável de ambiente.
    std::env::var("COMMERCECTRL_API_URL").unwrap_or_else(|_| {
        option_env!("COMMERCECTRL_API_URL")
            .filter(|v| !v.is_empty())
            .unwrap_or(if cfg!(debug_assertions) {
                "http://127.0.0.1:8088"
            } else {
                ""
            })
            .into()
    })
}

#[tauri::command]
fn cloud_generate_pix(amount_brl_cents: i64) -> Result<CloudPaymentIntent, String> {
    if amount_brl_cents <= 0 {
        return Err("Valor PIX inválido".into());
    }
    let api = cloud_api_url();
    if api.is_empty() {
        return Err("Backend de pagamentos não configurado".into());
    }
    let reference = format!("checkout-{}", uuid::Uuid::new_v4());
    let body = serde_json::json!({"external_reference":reference,"amount_brl_cents":amount_brl_cents,"description":"Venda no PDV CommerceCTRL"}).to_string();
    let response = cloud_request(ureq::post(&format!("{api}/api/v1/payments/pix")))
        .set("Content-Type", "application/json")
        .send_string(&body)
        .map_err(|e| format!("PIX automático indisponível: {e}"))?;
    serde_json::from_reader(response.into_reader()).map_err(|e| e.to_string())
}

#[tauri::command]
fn cloud_payment_status(id: String) -> Result<CloudPaymentIntent, String> {
    let api = cloud_api_url();
    if api.is_empty() {
        return Err("Backend de pagamentos não configurado".into());
    }
    let response = cloud_request(ureq::get(&format!("{api}/api/v1/payments/{id}")))
        .call()
        .map_err(|e| format!("Consulta PIX indisponível: {e}"))?;
    serde_json::from_reader(response.into_reader()).map_err(|e| e.to_string())
}

#[tauri::command]
fn cloud_sales_by_unit(from: i64, to: i64) -> Result<Vec<UnitSalesReport>, String> {
    let value = |name: &str, dev: &str| {
        std::env::var(name).unwrap_or_else(|_| {
            if cfg!(debug_assertions) {
                dev.into()
            } else {
                String::new()
            }
        })
    };
    let api = value("COMMERCECTRL_API_URL", "http://127.0.0.1:8088");
    let response = ureq::get(&format!(
        "{api}/api/v1/reports/sales-by-unit?from={from}&to={to}"
    ))
    .set(
        "X-Tenant-Id",
        &value(
            "COMMERCECTRL_TENANT_ID",
            "00000000-0000-0000-0000-000000000001",
        ),
    )
    .set(
        "X-Unit-Id",
        &value(
            "COMMERCECTRL_UNIT_ID",
            "00000000-0000-0000-0000-000000000101",
        ),
    )
    .set(
        "X-Terminal-Id",
        &value(
            "COMMERCECTRL_TERMINAL_ID",
            "00000000-0000-0000-0000-000000001001",
        ),
    )
    .set(
        "X-Terminal-Key",
        &value("COMMERCECTRL_TERMINAL_KEY", "commercectrl-dev-key"),
    )
    .timeout(std::time::Duration::from_secs(8))
    .call()
    .map_err(|e| format!("Relatório cloud indisponível: {e}"))?;
    serde_json::from_reader(response.into_reader()).map_err(|e| e.to_string())
}

#[tauri::command]
fn cloud_owner_overview(from: i64, to: i64) -> Result<OwnerOverview, String> {
    if from < 0 || to < from {
        return Err("Período inválido".into());
    }
    let api = cloud_api_url();
    if api.is_empty() {
        return Err("Backend do proprietário não configurado".into());
    }
    let tenant_id = std::env::var("COMMERCECTRL_TENANT_ID").unwrap_or_else(|_| {
        if cfg!(debug_assertions) {
            "00000000-0000-0000-0000-000000000001".into()
        } else {
            String::new()
        }
    });
    let owner_key = std::env::var("COMMERCECTRL_OWNER_KEY").unwrap_or_else(|_| {
        if cfg!(debug_assertions) {
            "commercectrl-owner-dev-key".into()
        } else {
            String::new()
        }
    });
    if tenant_id.is_empty() || owner_key.is_empty() {
        return Err("Credencial do proprietário ausente".into());
    }
    let response = ureq::get(&format!("{api}/api/v1/owner/overview?from={from}&to={to}"))
        .set("X-Tenant-Id", &tenant_id)
        .set("X-Owner-Key", &owner_key)
        .timeout(std::time::Duration::from_secs(8))
        .call()
        .map_err(|e| format!("Painel consolidado indisponível: {e}"))?;
    serde_json::from_reader(response.into_reader()).map_err(|e| e.to_string())
}

fn owner_request(request: ureq::Request) -> Result<ureq::Request, String> {
    let tenant_id = std::env::var("COMMERCECTRL_TENANT_ID")
        .unwrap_or_else(|_| "00000000-0000-0000-0000-000000000001".into());
    let owner_key = std::env::var("COMMERCECTRL_OWNER_KEY")
        .unwrap_or_else(|_| "commercectrl-owner-dev-key".into());
    if tenant_id.is_empty() || owner_key.is_empty() {
        return Err("Credencial do proprietário ausente".into());
    }
    Ok(request
        .set("X-Tenant-Id", &tenant_id)
        .set("X-Owner-Key", &owner_key)
        .timeout(std::time::Duration::from_secs(8)))
}

#[tauri::command]
fn cloud_owner_branding() -> Result<TenantBranding, String> {
    let api = cloud_api_url();
    let response = owner_request(ureq::get(&format!("{api}/api/v1/owner/branding")))?
        .call()
        .map_err(|e| format!("Identidade cloud indisponível: {e}"))?;
    serde_json::from_reader(response.into_reader()).map_err(|e| e.to_string())
}

#[tauri::command]
fn cloud_save_owner_branding(
    display_name: String,
    logo_data_url: Option<String>,
) -> Result<TenantBranding, String> {
    let api = cloud_api_url();
    let body =
        serde_json::json!({"display_name":display_name,"logo_data_url":logo_data_url}).to_string();
    let response = owner_request(ureq::put(&format!("{api}/api/v1/owner/branding")))?
        .set("Content-Type", "application/json")
        .send_string(&body)
        .map_err(|e| format!("Não foi possível salvar a identidade: {e}"))?;
    serde_json::from_reader(response.into_reader()).map_err(|e| e.to_string())
}

#[tauri::command]
fn local_display_link(state: tauri::State<'_, Arc<AppState>>) -> Result<LocalDisplayLink, String> {
    let token = state
        .db
        .get_setting("display_access_token")
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| {
            let value = uuid::Uuid::new_v4().simple().to_string();
            let _ = state.db.set_setting("display_access_token", &value);
            value
        });
    let address =
        std::env::var("COMMERCECTRL_DISPLAY_ADDR").unwrap_or_else(|_| "0.0.0.0:9002".into());
    let port = address
        .rsplit(':')
        .next()
        .and_then(|part| part.parse().ok())
        .unwrap_or(9002);
    Ok(LocalDisplayLink { url: format!("http://localhost:{port}/tv/{token}"), port,
        note: "Na TV, substitua localhost pelo IP local deste caixa. Esta URL é exclusiva da unidade atual.".into() })
}

#[tauri::command]
fn get_display_settings(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::server::DisplaySettings, String> {
    match state
        .db
        .get_setting("display_settings")
        .map_err(|e| e.to_string())?
    {
        Some(raw) => {
            serde_json::from_str(&raw).map_err(|_| "Configuração da TV corrompida".to_string())
        }
        None => Ok(crate::server::DisplaySettings::default()),
    }
}

#[tauri::command]
fn save_display_settings(
    state: tauri::State<'_, Arc<AppState>>,
    settings: crate::server::DisplaySettings,
) -> Result<crate::server::DisplaySettings, String> {
    if !matches!(settings.layout.as_str(), "promotions" | "video")
        || !(5..=120).contains(&settings.promotion_interval_seconds)
        || settings.radios.len() > 20
        || settings.youtube_id.as_deref().is_some_and(|id| {
            id.is_empty()
                || id.len() > 20
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
        })
        || settings.radios.iter().any(|radio| {
            radio.name.trim().is_empty()
                || radio.name.len() > 80
                || radio.url.len() > 500
                || !radio.url.starts_with("https://")
        })
        || settings
            .selected_radio_url
            .as_deref()
            .is_some_and(|url| !settings.radios.iter().any(|radio| radio.url == url))
    {
        return Err("Configuração do Modo TV inválida".into());
    }
    let raw = serde_json::to_string(&settings).map_err(|e| e.to_string())?;
    state
        .db
        .set_setting("display_settings", &raw)
        .map_err(|e| e.to_string())?;
    Ok(settings)
}

// ── Product commands ─────────────────────────────────────────

#[tauri::command]
fn search_product(
    state: tauri::State<'_, Arc<AppState>>,
    query: String,
) -> Result<Vec<Product>, String> {
    state
        .db
        .search(&query, 50)
        .map_err(|e| format!("busca falhou: {e}"))
}

#[tauri::command]
fn search_by_ean(
    state: tauri::State<'_, Arc<AppState>>,
    ean: String,
) -> Result<Option<Product>, String> {
    state
        .db
        .search_by_ean(&ean)
        .map_err(|e| format!("busca por EAN falhou: {e}"))
}

#[tauri::command]
fn list_products(
    state: tauri::State<'_, Arc<AppState>>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Product>, String> {
    state
        .db
        .list_products(limit.unwrap_or(200), offset.unwrap_or(0))
        .map_err(|e| format!("listar produtos: {e}"))
}

#[tauri::command]
fn save_product(
    state: tauri::State<'_, Arc<AppState>>,
    ean: String,
    part_number: String,
    description: String,
    brand: Option<String>,
    price_brl_cents: i64,
    stock_qty: f64,
    min_stock: f64,
    image_url: Option<String>,
) -> Result<Product, String> {
    state.require("catalog")?;
    if ean.trim().is_empty()
        || description.trim().is_empty()
        || price_brl_cents < 0
        || stock_qty < 0.0
        || min_stock < 0.0
    {
        return Err("Dados do produto inválidos".into());
    }
    state
        .db
        .upsert_product(
            ean.trim(),
            part_number.trim(),
            description.trim(),
            brand.as_deref(),
            price_brl_cents,
            stock_qty,
        )
        .map_err(|e| e.to_string())?;
    state
        .db
        .set_product_min_stock(ean.trim(), min_stock)
        .map_err(|e| e.to_string())?;
    if image_url
        .as_ref()
        .is_some_and(|value| value.len() > 700_000 || !value.starts_with("data:image/webp;base64,"))
    {
        return Err("A imagem deve ser WebP e ter no máximo 500 KB".into());
    }
    state
        .db
        .set_product_image(ean.trim(), image_url.as_deref())
        .map_err(|e| e.to_string())?;
    let item = state
        .db
        .search_by_ean(ean.trim())
        .map_err(|e| e.to_string())?
        .ok_or("Produto não encontrado")?;
    state
        .db
        .enqueue_outbox("product", "upsert", &serde_json::to_string(&item).unwrap())
        .map_err(|e| e.to_string())?;
    state.refresh_pending();
    Ok(item)
}

#[tauri::command]
fn adjust_stock(
    state: tauri::State<'_, Arc<AppState>>,
    ean: String,
    delta: f64,
) -> Result<f64, String> {
    state.require("inventory")?;
    let new_stock = state
        .db
        .adjust_stock(&ean, delta)
        .map_err(|e| format!("estoque: {e}"))?
        .ok_or_else(|| format!("produto {ean} não encontrado"))?;
    let payload = serde_json::json!({
        "ean": ean,
        "delta": delta,
        "new_stock_qty": new_stock,
    })
    .to_string();
    state
        .db
        .enqueue_outbox("stock_movement", "update", &payload)
        .map_err(|e| format!("outbox: {e}"))?;
    state.refresh_pending();
    Ok(new_stock)
}

#[tauri::command]
fn archive_product(state: tauri::State<'_, Arc<AppState>>, ean: String) -> Result<(), String> {
    state.require("catalog")?;
    let mut item = state
        .db
        .search_by_ean(ean.trim())
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Produto não encontrado".to_string())?;
    if !state
        .db
        .set_product_active(ean.trim(), false)
        .map_err(|e| e.to_string())?
    {
        return Err("Produto não encontrado".into());
    }
    item.active = false;
    state
        .db
        .enqueue_outbox("product", "archive", &serde_json::to_string(&item).unwrap())
        .map_err(|e| e.to_string())?;
    state.refresh_pending();
    Ok(())
}

#[tauri::command]
fn create_purchase_order(
    state: tauri::State<'_, Arc<AppState>>,
    ean: String,
    quantity: f64,
) -> Result<i64, String> {
    state.require("inventory")?;
    if quantity <= 0.0 {
        return Err("Quantidade inválida".into());
    }
    let id = state
        .db
        .create_purchase_order(&ean, quantity)
        .map_err(|e| e.to_string())?;
    let payload = serde_json::json!({"local_id":id,"product_ean":ean,"quantity":quantity,"status":"Pendente"}).to_string();
    state
        .db
        .enqueue_outbox("purchase_order", "create", &payload)
        .map_err(|e| e.to_string())?;
    state.refresh_pending();
    Ok(id)
}

// ── Sale commands ────────────────────────────────────────────

#[tauri::command]
fn record_sale(
    state: tauri::State<'_, Arc<AppState>>,
    items: Vec<SaleItemIn>,
    payment: PaymentIn,
    terminal_id: Option<String>,
    consumer_document: Option<String>,
) -> Result<SaleOut, String> {
    state.require("pos")?;
    let license = crate::license::status(&state.db);
    if !license.allowed_to_sell {
        return Err(format!("Licença bloqueada: {}", license.message));
    }
    let sale = state
        .db
        .record_sale_with_fiscal(
            &items,
            &payment,
            terminal_id.as_deref().unwrap_or("default"),
            consumer_document
                .as_deref()
                .filter(|value| !value.trim().is_empty()),
        )
        .map_err(|e| format!("falha ao gravar venda: {e}"))?;
    state.refresh_pending();
    let _ = state.publish(serde_json::to_string(&sale).unwrap_or_default());
    Ok(sale)
}

#[tauri::command]
fn license_status(state: tauri::State<'_, Arc<AppState>>) -> crate::license::LicenseStatus {
    crate::license::status(&state.db)
}

#[tauri::command]
fn import_license(
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
) -> Result<crate::license::LicenseStatus, String> {
    crate::license::import_file(&state.db, std::path::Path::new(&path))
}

#[tauri::command]
fn scan_license_media(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::license::LicenseStatus, String> {
    crate::license::scan_removable(&state.db)
}

#[tauri::command]
fn refresh_license(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::license::LicenseStatus, String> {
    let current = crate::license::status(&state.db);
    let value = |name: &str, dev: &str| {
        std::env::var(name).unwrap_or_else(|_| {
            if cfg!(debug_assertions) {
                dev.into()
            } else {
                String::new()
            }
        })
    };
    let api = value("COMMERCECTRL_API_URL", "http://127.0.0.1:8088");
    if api.is_empty() {
        return Err("backend de licenciamento não configurado".into());
    }
    let body = serde_json::json!({"installation_id": current.installation_id}).to_string();
    let response = ureq::post(&format!("{api}/api/v1/licenses/renew"))
        .set("Content-Type", "application/json")
        .set(
            "X-Tenant-Id",
            &value(
                "COMMERCECTRL_TENANT_ID",
                "00000000-0000-0000-0000-000000000001",
            ),
        )
        .set(
            "X-Unit-Id",
            &value(
                "COMMERCECTRL_UNIT_ID",
                "00000000-0000-0000-0000-000000000101",
            ),
        )
        .set(
            "X-Terminal-Id",
            &value(
                "COMMERCECTRL_TERMINAL_ID",
                "00000000-0000-0000-0000-000000001001",
            ),
        )
        .set(
            "X-Terminal-Key",
            &value("COMMERCECTRL_TERMINAL_KEY", "commercectrl-dev-key"),
        )
        .timeout(std::time::Duration::from_secs(8))
        .send_string(&body)
        .map_err(|e| format!("renovação de licença indisponível: {e}"))?;
    let raw: serde_json::Value =
        serde_json::from_reader(response.into_reader()).map_err(|e| e.to_string())?;
    crate::license::import_token(&state.db, &raw.to_string(), Some("online"))
}

// ── Outbox commands ──────────────────────────────────────────

#[tauri::command]
fn pending_outbox(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::OutboxRow>, String> {
    state.outbox.pending().map_err(|e| format!("outbox: {e}"))
}

#[tauri::command]
fn sync_now(state: tauri::State<'_, Arc<AppState>>) -> Result<usize, String> {
    if !state.outbox.is_configured() {
        state.set_online(false);
        state.refresh_pending();
        return Err("sincronização em nuvem ainda não configurada".into());
    }
    let n = state
        .outbox
        .sync_pending()
        .map_err(|e| format!("sync: {e}"))?;
    state.set_online(true);
    state.refresh_pending();
    Ok(n)
}

#[tauri::command]
fn sync_status(state: tauri::State<'_, Arc<AppState>>) -> Result<crate::state::SyncInfo, String> {
    Ok(state.status())
}

#[tauri::command]
fn dashboard_summary(state: tauri::State<'_, Arc<AppState>>) -> Result<DashboardSummary, String> {
    state.db.dashboard_summary().map_err(|e| e.to_string())
}

#[tauri::command]
fn list_employees(state: tauri::State<'_, Arc<AppState>>) -> Result<Vec<Employee>, String> {
    state.db.list_employees().map_err(|e| e.to_string())
}

#[tauri::command]
fn list_time_entries(
    state: tauri::State<'_, Arc<AppState>>,
    limit: i64,
) -> Result<Vec<TimeEntry>, String> {
    state.db.list_time_entries(limit).map_err(|e| e.to_string())
}

#[tauri::command]
fn record_time_entry(
    state: tauri::State<'_, Arc<AppState>>,
    employee_id: i64,
    event_type: String,
    note: Option<String>,
) -> Result<TimeEntry, String> {
    if !matches!(
        event_type.as_str(),
        "clock_in" | "break_start" | "break_end" | "clock_out"
    ) {
        return Err("Tipo de registro de ponto inválido".into());
    }
    let item = state
        .db
        .record_time_entry(employee_id, &event_type, note.as_deref())
        .map_err(|e| e.to_string())?;
    state
        .db
        .enqueue_outbox(
            "time_entry",
            "create",
            &serde_json::to_string(&item).unwrap(),
        )
        .map_err(|e| e.to_string())?;
    state.refresh_pending();
    Ok(item)
}

#[tauri::command]
fn save_employee(
    state: tauri::State<'_, Arc<AppState>>,
    name: String,
    role: String,
    status: String,
) -> Result<Employee, String> {
    state.require("employees")?;
    let item = state
        .db
        .save_employee(name.trim(), role.trim(), &status)
        .map_err(|e| e.to_string())?;
    state
        .db
        .enqueue_outbox("employee", "create", &serde_json::to_string(&item).unwrap())
        .map_err(|e| e.to_string())?;
    state.refresh_pending();
    Ok(item)
}

#[tauri::command]
fn provision_employee_login(
    state: tauri::State<'_, Arc<AppState>>,
    employee_id: i64,
    username: String,
    role: String,
    temporary_password: String,
) -> Result<(), String> {
    state.require("employees")?;
    if username.trim().len() < 3
        || !matches!(
            role.as_str(),
            "admin" | "manager" | "cashier" | "stock" | "finance" | "viewer"
        )
    {
        return Err("Usuário ou perfil inválido".into());
    }
    let employee = state
        .db
        .list_employees()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|item| item.id == employee_id && item.status == "Ativo")
        .ok_or_else(|| "Funcionário ativo não encontrado".to_string())?;
    state
        .db
        .provision_user(
            employee_id,
            &employee.name,
            username.trim(),
            &role,
            &password_hash(&temporary_password)?,
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn update_employee(
    state: tauri::State<'_, Arc<AppState>>,
    id: i64,
    name: String,
    role: String,
    status: String,
) -> Result<Employee, String> {
    if name.trim().is_empty()
        || role.trim().is_empty()
        || !matches!(status.as_str(), "Ativo" | "Inativo")
    {
        return Err("Dados do funcionário inválidos".into());
    }
    let item = state
        .db
        .update_employee(id, name.trim(), role.trim(), &status)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Funcionário não encontrado".to_string())?;
    state
        .db
        .enqueue_outbox("employee", "update", &serde_json::to_string(&item).unwrap())
        .map_err(|e| e.to_string())?;
    state.refresh_pending();
    Ok(item)
}

#[tauri::command]
fn list_expenses(state: tauri::State<'_, Arc<AppState>>) -> Result<Vec<Expense>, String> {
    state.db.list_expenses().map_err(|e| e.to_string())
}

#[tauri::command]
fn save_expense(
    state: tauri::State<'_, Arc<AppState>>,
    description: String,
    category: String,
    amount_brl_cents: i64,
    due_date: String,
    status: String,
) -> Result<Expense, String> {
    state.require("finance")?;
    if amount_brl_cents <= 0 {
        return Err("O valor deve ser maior que zero".into());
    }
    let item = state
        .db
        .save_expense(
            description.trim(),
            category.trim(),
            amount_brl_cents,
            &due_date,
            &status,
        )
        .map_err(|e| e.to_string())?;
    state
        .db
        .enqueue_outbox("expense", "create", &serde_json::to_string(&item).unwrap())
        .map_err(|e| e.to_string())?;
    state.refresh_pending();
    Ok(item)
}

#[tauri::command]
fn update_expense(
    state: tauri::State<'_, Arc<AppState>>,
    id: i64,
    description: String,
    category: String,
    amount_brl_cents: i64,
    due_date: String,
    status: String,
) -> Result<Expense, String> {
    if description.trim().is_empty()
        || category.trim().is_empty()
        || amount_brl_cents <= 0
        || !matches!(status.as_str(), "Pago" | "Pendente")
    {
        return Err("Dados da despesa inválidos".into());
    }
    let item = state
        .db
        .update_expense(
            id,
            description.trim(),
            category.trim(),
            amount_brl_cents,
            &due_date,
            &status,
        )
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Despesa não encontrada".to_string())?;
    state
        .db
        .enqueue_outbox("expense", "update", &serde_json::to_string(&item).unwrap())
        .map_err(|e| e.to_string())?;
    state.refresh_pending();
    Ok(item)
}

#[tauri::command]
fn list_suppliers(state: tauri::State<'_, Arc<AppState>>) -> Result<Vec<Supplier>, String> {
    state.db.list_suppliers().map_err(|e| e.to_string())
}

#[tauri::command]
fn save_supplier(
    state: tauri::State<'_, Arc<AppState>>,
    name: String,
    document: Option<String>,
    phone: Option<String>,
    status: String,
) -> Result<Supplier, String> {
    state.require("suppliers")?;
    let item = state
        .db
        .save_supplier(name.trim(), document.as_deref(), phone.as_deref(), &status)
        .map_err(|e| e.to_string())?;
    state
        .db
        .enqueue_outbox("supplier", "create", &serde_json::to_string(&item).unwrap())
        .map_err(|e| e.to_string())?;
    state.refresh_pending();
    Ok(item)
}

#[tauri::command]
fn update_supplier(
    state: tauri::State<'_, Arc<AppState>>,
    id: i64,
    name: String,
    document: Option<String>,
    phone: Option<String>,
    status: String,
) -> Result<Supplier, String> {
    if name.trim().is_empty() || !matches!(status.as_str(), "Ativo" | "Inativo") {
        return Err("Dados do fornecedor inválidos".into());
    }
    let item = state
        .db
        .update_supplier(
            id,
            name.trim(),
            document.as_deref(),
            phone.as_deref(),
            &status,
        )
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Fornecedor não encontrado".to_string())?;
    state
        .db
        .enqueue_outbox("supplier", "update", &serde_json::to_string(&item).unwrap())
        .map_err(|e| e.to_string())?;
    state.refresh_pending();
    Ok(item)
}

#[tauri::command]
fn list_promotions(
    state: tauri::State<'_, Arc<AppState>>,
    include_inactive: Option<bool>,
) -> Result<Vec<Promotion>, String> {
    state
        .db
        .list_promotions(include_inactive.unwrap_or(false))
        .map_err(|e| e.to_string())
}

fn enqueue_status(
    state: &Arc<AppState>,
    entity: &str,
    id: i64,
    field: &str,
    value: serde_json::Value,
) -> Result<(), String> {
    let mut payload_map = serde_json::Map::new();
    payload_map.insert("id".into(), id.into());
    payload_map.insert(field.into(), value);
    let payload = serde_json::Value::Object(payload_map).to_string();
    state
        .db
        .enqueue_outbox(entity, "update", &payload)
        .map_err(|e| e.to_string())?;
    state.refresh_pending();
    Ok(())
}

#[tauri::command]
fn set_employee_status(
    state: tauri::State<'_, Arc<AppState>>,
    id: i64,
    status: String,
) -> Result<(), String> {
    state.require("employees")?;
    if !matches!(status.as_str(), "Ativo" | "Inativo") {
        return Err("Status inválido".into());
    }
    if !state
        .db
        .set_employee_status(id, &status)
        .map_err(|e| e.to_string())?
    {
        return Err("Funcionário não encontrado".into());
    }
    enqueue_status(state.inner(), "employee", id, "status", status.into())
}

#[tauri::command]
fn set_expense_status(
    state: tauri::State<'_, Arc<AppState>>,
    id: i64,
    status: String,
) -> Result<(), String> {
    state.require("finance")?;
    if !matches!(status.as_str(), "Pago" | "Pendente") {
        return Err("Status inválido".into());
    }
    if !state
        .db
        .set_expense_status(id, &status)
        .map_err(|e| e.to_string())?
    {
        return Err("Despesa não encontrada".into());
    }
    enqueue_status(state.inner(), "expense", id, "status", status.into())
}

#[tauri::command]
fn set_supplier_status(
    state: tauri::State<'_, Arc<AppState>>,
    id: i64,
    status: String,
) -> Result<(), String> {
    state.require("suppliers")?;
    if !matches!(status.as_str(), "Ativo" | "Inativo") {
        return Err("Status inválido".into());
    }
    if !state
        .db
        .set_supplier_status(id, &status)
        .map_err(|e| e.to_string())?
    {
        return Err("Fornecedor não encontrado".into());
    }
    enqueue_status(state.inner(), "supplier", id, "status", status.into())
}

#[tauri::command]
fn set_promotion_active(
    state: tauri::State<'_, Arc<AppState>>,
    id: i64,
    active: bool,
) -> Result<(), String> {
    state.require("promotions")?;
    if !state
        .db
        .set_promotion_active(id, active)
        .map_err(|e| e.to_string())?
    {
        return Err("Promoção não encontrada".into());
    }
    enqueue_status(state.inner(), "promotion", id, "active", active.into())
}

#[tauri::command]
fn save_promotion(
    state: tauri::State<'_, Arc<AppState>>,
    title: String,
    subtitle: Option<String>,
    price_label: String,
    active: bool,
) -> Result<Promotion, String> {
    state.require("promotions")?;
    if title.trim().is_empty() || price_label.trim().is_empty() {
        return Err("Título e chamada de preço são obrigatórios".into());
    }
    let item = state
        .db
        .save_promotion(
            title.trim(),
            subtitle.as_deref(),
            price_label.trim(),
            active,
        )
        .map_err(|e| e.to_string())?;
    state
        .db
        .enqueue_outbox(
            "promotion",
            "create",
            &serde_json::to_string(&item).unwrap(),
        )
        .map_err(|e| e.to_string())?;
    state.refresh_pending();
    Ok(item)
}

#[tauri::command]
fn update_promotion(
    state: tauri::State<'_, Arc<AppState>>,
    id: i64,
    title: String,
    subtitle: Option<String>,
    price_label: String,
    active: bool,
) -> Result<Promotion, String> {
    state.require("promotions")?;
    if title.trim().is_empty() || price_label.trim().is_empty() {
        return Err("Título e chamada de preço são obrigatórios".into());
    }
    let item = state
        .db
        .update_promotion(
            id,
            title.trim(),
            subtitle.as_deref(),
            price_label.trim(),
            active,
        )
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Promoção não encontrada".to_string())?;
    state
        .db
        .enqueue_outbox(
            "promotion",
            "update",
            &serde_json::to_string(&item).unwrap(),
        )
        .map_err(|e| e.to_string())?;
    state.refresh_pending();
    Ok(item)
}

#[tauri::command]
fn current_payment_totals(state: tauri::State<'_, Arc<AppState>>) -> Result<PaymentTotals, String> {
    state.db.current_payment_totals().map_err(|e| e.to_string())
}

#[tauri::command]
fn sales_report(
    state: tauri::State<'_, Arc<AppState>>,
    from: i64,
    to: i64,
    payment_method: Option<String>,
    terminal_id: Option<String>,
    product_query: Option<String>,
) -> Result<Vec<SalesReportRow>, String> {
    if to <= from {
        return Err("Período inválido".into());
    }
    state
        .db
        .sales_report(
            from,
            to,
            payment_method.as_deref(),
            terminal_id.as_deref(),
            product_query.as_deref(),
        )
        .map_err(|e| e.to_string())
}

// ── Payment commands ─────────────────────────────────────────

#[tauri::command]
fn generate_pix(
    _state: tauri::State<'_, Arc<AppState>>,
    amount_brl_cents: i64,
    extra: Option<serde_json::Value>,
) -> Result<crate::payments::PaymentResult, String> {
    Ok(crate::payments::provider_for("pix").authorize(amount_brl_cents, extra.as_ref()))
}

// ── Cash commands ────────────────────────────────────────────

#[tauri::command]
fn cash_status(state: tauri::State<'_, Arc<AppState>>) -> Result<crate::db::CashStatus, String> {
    state.db.cash_status().map_err(|e| format!("caixa: {e}"))
}

#[tauri::command]
fn cash_open(
    state: tauri::State<'_, Arc<AppState>>,
    opening_brl_cents: i64,
) -> Result<crate::db::CashStatus, String> {
    let status = state
        .db
        .cash_open(opening_brl_cents)
        .map_err(|e| format!("abrir caixa: {e}"))?;
    let payload =
        serde_json::json!({"local_id": status.session_id, "action": "open", "status": status});
    state
        .db
        .enqueue_outbox("cash_session", "create", &payload.to_string())
        .map_err(|e| e.to_string())?;
    state.refresh_pending();
    Ok(status)
}

#[tauri::command]
fn cash_close(
    state: tauri::State<'_, Arc<AppState>>,
    closing_brl_cents: i64,
) -> Result<crate::db::CashStatus, String> {
    let status = state
        .db
        .cash_close(closing_brl_cents)
        .map_err(|e| format!("fechar caixa: {e}"))?;
    let payload =
        serde_json::json!({"local_id": status.session_id, "action": "close", "status": status});
    state
        .db
        .enqueue_outbox("cash_session", "update", &payload.to_string())
        .map_err(|e| e.to_string())?;
    state.refresh_pending();
    Ok(status)
}

#[tauri::command]
fn cash_sangria(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: i64,
    amount_brl_cents: i64,
    description: Option<String>,
    operator: Option<String>,
) -> Result<crate::db::CashMovement, String> {
    let movement = financial::CashierManager::sangria(
        &state.db,
        session_id,
        amount_brl_cents,
        description.as_deref(),
        operator.as_deref(),
    )?;
    state
        .db
        .enqueue_outbox(
            "cash_movement",
            "create",
            &serde_json::to_string(&movement).unwrap(),
        )
        .map_err(|e| e.to_string())?;
    state.refresh_pending();
    Ok(movement)
}

#[tauri::command]
fn cash_suprimento(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: i64,
    amount_brl_cents: i64,
    description: Option<String>,
    operator: Option<String>,
) -> Result<crate::db::CashMovement, String> {
    let movement = financial::CashierManager::suprimento(
        &state.db,
        session_id,
        amount_brl_cents,
        description.as_deref(),
        operator.as_deref(),
    )?;
    state
        .db
        .enqueue_outbox(
            "cash_movement",
            "create",
            &serde_json::to_string(&movement).unwrap(),
        )
        .map_err(|e| e.to_string())?;
    state.refresh_pending();
    Ok(movement)
}

#[tauri::command]
fn cash_fechamento_cego(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: i64,
    counted: CashCounted,
) -> Result<crate::db::FechamentoReport, String> {
    financial::CashierManager::fechamento_cego(&state.db, session_id, counted)
}

// ── Print commands ───────────────────────────────────────────

#[tauri::command]
async fn print_receipt(
    state: tauri::State<'_, Arc<AppState>>,
    sale_uuid: String,
    items: Vec<(String, i64)>,
    total_brl_cents: i64,
    store_name: Option<String>,
    payment_method: Option<String>,
    consumer_document: Option<String>,
    logo_data_url: Option<String>,
) -> Result<(), String> {
    state
        .printer
        .submit(PrintJob::Receipt {
            sale_uuid,
            items,
            total_brl_cents,
            store_name,
            payment_method,
            consumer_document,
            logo_data_url,
        })
        .await
}

#[tauri::command]
async fn print_text(state: tauri::State<'_, Arc<AppState>>, text: String) -> Result<(), String> {
    state.printer.submit(PrintJob::Text { text }).await
}

#[tauri::command]
fn print_status(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::printer::PrintStatus, String> {
    Ok(state.printer.status())
}

#[tauri::command]
fn list_printers() -> Result<Vec<crate::printer::PrinterInfo>, String> {
    crate::printer::list_windows_printers()
}

#[tauri::command]
fn printer_config(state: tauri::State<'_, Arc<AppState>>) -> crate::printer::PrinterConfig {
    state.printer.config()
}

#[tauri::command]
fn save_printer_config(
    state: tauri::State<'_, Arc<AppState>>,
    config: crate::printer::PrinterConfig,
) -> Result<(), String> {
    state.printer.save_config(config)
}

#[tauri::command]
fn print_retry(state: tauri::State<'_, Arc<AppState>>, spool_id: i64) -> Result<(), String> {
    let prints = state.db.list_pending_prints().map_err(|e| e.to_string())?;
    let job = prints.iter().find(|p| p.id == spool_id);
    match job {
        Some(j) => {
            let print_job: PrintJob =
                serde_json::from_str(&j.receipt_json).map_err(|e| e.to_string())?;
            let printer = state.printer.clone();
            let db = state.db.clone();
            let spool_id = j.id;
            tauri::async_runtime::spawn(async move {
                match printer.submit(print_job).await {
                    Ok(()) => {
                        let _ = db.mark_print_sent(spool_id);
                    }
                    Err(e) => {
                        let _ = db.mark_print_failed(spool_id, &e);
                    }
                }
            });
            Ok(())
        }
        None => Err(format!("Spool job {spool_id} não encontrado")),
    }
}

// ── Barcode ──────────────────────────────────────────────────

#[tauri::command]
fn emit_barcode(app: tauri::AppHandle, code: String) -> Result<(), String> {
    let payload = BarcodePayload {
        code: code.clone(),
        at_ms: chrono::Utc::now().timestamp_millis() as u64,
    };
    app.emit("barcode-scanned", &payload)
        .map_err(|e| format!("emit: {e}"))?;
    let st = app.state::<Arc<AppState>>();
    let _ = st
        .events
        .send(serde_json::to_string(&payload).unwrap_or_default());
    Ok(())
}

// ── Customer commands ────────────────────────────────────────

#[tauri::command]
fn search_customers(
    state: tauri::State<'_, Arc<AppState>>,
    query: String,
) -> Result<Vec<Customer>, String> {
    state
        .db
        .search_customers(&query)
        .map_err(|e| format!("buscar clientes: {e}"))
}

#[tauri::command]
fn list_customers(
    state: tauri::State<'_, Arc<AppState>>,
    limit: Option<i64>,
) -> Result<Vec<Customer>, String> {
    state
        .db
        .list_customers(limit.unwrap_or(50))
        .map_err(|e| format!("listar clientes: {e}"))
}

#[tauri::command]
fn upsert_customer(
    state: tauri::State<'_, Arc<AppState>>,
    name: String,
    cpf_cnpj: Option<String>,
    phone: Option<String>,
    email: Option<String>,
) -> Result<Customer, String> {
    let item = state
        .db
        .upsert_customer(
            &name,
            cpf_cnpj.as_deref(),
            phone.as_deref(),
            email.as_deref(),
        )
        .map_err(|e| format!("cliente: {e}"))?;
    state
        .db
        .enqueue_outbox("customer", "upsert", &serde_json::to_string(&item).unwrap())
        .map_err(|e| e.to_string())?;
    state.refresh_pending();
    Ok(item)
}

#[tauri::command]
fn update_customer(
    state: tauri::State<'_, Arc<AppState>>,
    id: i64,
    name: String,
    cpf_cnpj: Option<String>,
    phone: Option<String>,
    email: Option<String>,
) -> Result<Customer, String> {
    let item = state
        .db
        .update_customer(
            id,
            name.trim(),
            cpf_cnpj.as_deref(),
            phone.as_deref(),
            email.as_deref(),
        )
        .map_err(|e| format!("atualizar cliente: {e}"))?;
    state
        .db
        .enqueue_outbox("customer", "update", &serde_json::to_string(&item).unwrap())
        .map_err(|e| e.to_string())?;
    state.refresh_pending();
    Ok(item)
}

#[tauri::command]
fn add_customer_points(
    state: tauri::State<'_, Arc<AppState>>,
    customer_id: i64,
    points: i64,
    sale_uuid: Option<String>,
    description: Option<String>,
) -> Result<(), String> {
    state
        .db
        .add_points(
            customer_id,
            points,
            sale_uuid.as_deref(),
            description.as_deref(),
        )
        .map_err(|e| format!("adicionar pontos: {e}"))
}

#[tauri::command]
fn redeem_customer_points(
    state: tauri::State<'_, Arc<AppState>>,
    customer_id: i64,
    points: i64,
    description: Option<String>,
) -> Result<i64, String> {
    state
        .db
        .redeem_points(customer_id, points, description.as_deref())
        .map_err(|e| format!("resgatar pontos: {e}"))
}

// ── NFe B2B command ──────────────────────────────────────────

#[tauri::command]
fn emit_nfe(
    _state: tauri::State<'_, Arc<AppState>>,
    _params: crate::fiscal::NfeParams,
) -> Result<crate::fiscal::NfeResult, String> {
    Err("Emissão fiscal bloqueada até configurar certificado A1, CSC e homologação SEFAZ-SP; a venda foi preservada na fila fiscal".into())
}

// ── Logger ───────────────────────────────────────────────────

fn init_logger() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init();
}

// ── Main ─────────────────────────────────────────────────────

fn main() {
    init_logger();

    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            std::fs::create_dir_all(&app_dir)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

            let db_path = app_dir.join("pdv.db");
            let restore_marker = app_dir.join("restore.pending");
            if let Ok(staged) = std::fs::read_to_string(&restore_marker) {
                let staged = std::path::PathBuf::from(staged.trim());
                db::Db::validate_backup(&staged)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                if db_path.exists() {
                    let _ = std::fs::copy(&db_path, app_dir.join("pdv.before-restore.db"));
                }
                std::fs::copy(&staged, &db_path)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
                let _ = std::fs::remove_file(&restore_marker);
                let _ = std::fs::remove_file(staged);
            }
            let db = Arc::new(
                db::Db::open(&db_path)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?,
            );
            let outbox = Arc::new(sync::Outbox::new(db.clone()));
            let print_worker = Arc::new(printer::PrintWorker::start(
                db.clone(),
                app.handle().clone(),
                app_dir.join("printer.json"),
            ));
            let st = Arc::new(AppState::new(db, outbox, print_worker));

            let api_url = std::env::var("COMMERCECTRL_API_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8088".into());
            let server_addr = std::env::var("COMMERCECTRL_LOCAL_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:9001".into());

            let st_server = st.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = server::serve(st_server.as_ref().clone(), &server_addr).await {
                    log::error!("Servidor local Axum falhou: {e}");
                }
            });

            let display_addr = std::env::var("COMMERCECTRL_DISPLAY_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:9002".into());
            let st_display = st.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) =
                    server::serve_display(st_display.as_ref().clone(), &display_addr).await
                {
                    log::error!("Servidor do Modo TV falhou: {e}");
                }
            });

            let st_worker = st.clone();
            std::thread::Builder::new()
                .name("pdv-sync".into())
                .spawn(move || sync::worker(st_worker, api_url))
                .ok();

            let st_hook = st.clone();
            if let Err(e) = win32::install_barcode_hook(st_hook) {
                log::warn!("Barcode hook: {e}");
            }

            app.manage(st);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            login_employee,
            current_session,
            logout_employee,
            change_own_password,
            create_terminal_backup,
            schedule_terminal_restore,
            kiosk_admin_status,
            configure_kiosk_admin_pin,
            verify_admin_pin,
            exit_kiosk_as_admin,
            search_product,
            search_by_ean,
            record_sale,
            pending_outbox,
            sync_now,
            list_products,
            save_product,
            adjust_stock,
            archive_product,
            create_purchase_order,
            sync_status,
            dashboard_summary,
            list_employees,
            list_time_entries,
            record_time_entry,
            save_employee,
            provision_employee_login,
            update_employee,
            set_employee_status,
            list_expenses,
            save_expense,
            update_expense,
            set_expense_status,
            list_suppliers,
            save_supplier,
            update_supplier,
            set_supplier_status,
            list_promotions,
            save_promotion,
            update_promotion,
            set_promotion_active,
            current_payment_totals,
            sales_report,
            cloud_sales_by_unit,
            cloud_owner_overview,
            cloud_owner_branding,
            cloud_save_owner_branding,
            local_display_link,
            get_display_settings,
            save_display_settings,
            cloud_generate_pix,
            cloud_payment_status,
            generate_pix,
            cash_status,
            cash_open,
            cash_close,
            cash_sangria,
            cash_suprimento,
            cash_fechamento_cego,
            print_receipt,
            print_text,
            print_status,
            list_printers,
            printer_config,
            save_printer_config,
            print_retry,
            emit_barcode,
            search_customers,
            list_customers,
            upsert_customer,
            update_customer,
            add_customer_points,
            redeem_customer_points,
            emit_nfe,
            license_status,
            import_license,
            scan_license_media,
            refresh_license
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<Arc<AppState>>();
                if !state.kiosk_exit_is_authorized() {
                    api.prevent_close();
                    let _ = window.emit("kiosk-exit-requested", ());
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("erro ao executar o PDV");
}

#[cfg(test)]
mod command_tests {
    use super::*;

    #[test]
    fn every_role_has_only_expected_permissions() {
        assert_eq!(permissions_for("admin"), vec!["*"]);
        assert_eq!(permissions_for("superadmin"), vec!["*"]);
        assert!(permissions_for("manager").contains(&"settings".to_string()));
        assert!(permissions_for("cashier").contains(&"pos".to_string()));
        assert!(!permissions_for("cashier").contains(&"settings".to_string()));
        assert!(permissions_for("stock").contains(&"inventory".to_string()));
        assert!(permissions_for("finance").contains(&"finance".to_string()));
        assert_eq!(permissions_for("unknown"), vec!["time"]);
    }

    #[test]
    fn passwords_are_argon2_and_minimum_length_is_enforced() {
        assert!(password_hash("12345").is_err());
        let encoded = password_hash("senha-forte-123").unwrap();
        let parsed = PasswordHash::new(&encoded).unwrap();
        assert!(Argon2::default()
            .verify_password(b"senha-forte-123", &parsed)
            .is_ok());
        assert!(Argon2::default()
            .verify_password(b"incorreta", &parsed)
            .is_err());
    }

    #[test]
    fn kiosk_password_accepts_passphrases_and_rejects_unsafe_lengths() {
        assert!(validate_kiosk_pin("123456").is_ok());
        assert!(validate_kiosk_pin("senha forte").is_ok());
        assert!(validate_kiosk_pin("12345").is_err());
        assert!(validate_kiosk_pin(&"x".repeat(129)).is_err());
    }
}
