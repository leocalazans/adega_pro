use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde::Serialize;

use crate::db::{PaymentIn, Product, SaleItemIn, SaleOut};
use crate::state::AppState;

const TV_HTML: &str = include_str!("tv.html");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadioStation {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplaySettings {
    pub layout: String,
    pub youtube_id: Option<String>,
    pub radios: Vec<RadioStation>,
    pub selected_radio_url: Option<String>,
    pub promotion_interval_seconds: u32,
}
impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            layout: "promotions".into(),
            youtube_id: None,
            radios: vec![
                RadioStation {
                    name: "Radio Paradise".into(),
                    url: "https://stream.radioparadise.com/mp3-192".into(),
                },
                RadioStation {
                    name: "FIP".into(),
                    url: "https://icecast.radiofrance.fr/fip-hifi.aac".into(),
                },
            ],
            selected_radio_url: Some("https://stream.radioparadise.com/mp3-192".into()),
            promotion_interval_seconds: 10,
        }
    }
}
#[derive(Serialize)]
struct TvPayload {
    settings: DisplaySettings,
    promotions: Vec<crate::db::Promotion>,
}
fn display_settings(st: &AppState) -> Result<DisplaySettings, (StatusCode, String)> {
    let raw = st.db.get_setting("display_settings").map_err(err_500)?;
    raw.map(|value| {
        serde_json::from_str(&value).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Configuração da TV corrompida".into(),
            )
        })
    })
    .transpose()
    .map(|value| value.unwrap_or_default())
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
    service: &'static str,
    now_ms: i64,
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok",
        service: "autocontrol-pdv",
        now_ms: chrono::Utc::now().timestamp_millis(),
    })
}

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
    ean: Option<String>,
}

#[derive(Deserialize)]
struct ListQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Deserialize)]
struct ProductUpsert {
    ean: String,
    part_number: String,
    description: String,
    brand: Option<String>,
    price_brl_cents: i64,
    stock_qty: f64,
}

async fn list_products(
    State(st): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Product>>, (StatusCode, String)> {
    st.db
        .list_products(q.limit.unwrap_or(200), q.offset.unwrap_or(0))
        .map(Json)
        .map_err(err_500)
}

async fn upsert_product(
    State(st): State<AppState>,
    Json(p): Json<ProductUpsert>,
) -> Result<StatusCode, (StatusCode, String)> {
    st.db
        .upsert_product(
            &p.ean,
            &p.part_number,
            &p.description,
            p.brand.as_deref(),
            p.price_brl_cents,
            p.stock_qty,
        )
        .map_err(err_500)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn sync_status(
    State(st): State<AppState>,
) -> Result<Json<crate::state::SyncInfo>, (StatusCode, String)> {
    Ok(Json(st.status()))
}

async fn search(
    State(st): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<Vec<Product>>, (StatusCode, String)> {
    if let Some(ean) = q.ean {
        let found = st.db.search_by_ean(&ean).map_err(err_500)?;
        return Ok(Json(found.into_iter().collect()));
    }
    let q = q.q.unwrap_or_default();
    if q.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "parâmetro 'q' obrigatório".into()));
    }
    st.db.search(&q, 50).map(Json).map_err(err_500)
}

async fn by_ean(
    State(st): State<AppState>,
    Path(ean): Path<String>,
) -> Result<Json<Product>, (StatusCode, String)> {
    match st.db.search_by_ean(&ean).map_err(err_500)? {
        Some(p) => Ok(Json(p)),
        None => Err((
            StatusCode::NOT_FOUND,
            format!("produto {ean} não encontrado"),
        )),
    }
}

#[derive(Deserialize)]
struct SaleRequest {
    items: Vec<SaleItemIn>,
    payment: PaymentIn,
    #[serde(default)]
    terminal_id: Option<String>,
    #[serde(default)]
    consumer_document: Option<String>,
}

async fn create_sale(
    State(st): State<AppState>,
    Json(req): Json<SaleRequest>,
) -> Result<Json<SaleOut>, (StatusCode, String)> {
    let license = crate::license::status(&st.db);
    if !license.allowed_to_sell {
        return Err((StatusCode::PAYMENT_REQUIRED, license.message));
    }
    if req.items.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "venda sem itens".into()));
    }
    let sale = st
        .db
        .record_sale_with_fiscal(
            &req.items,
            &req.payment,
            req.terminal_id.as_deref().unwrap_or("default"),
            req.consumer_document.as_deref(),
        )
        .map_err(err_500)?;
    let _ = st
        .events
        .send(serde_json::to_string(&sale).unwrap_or_default());
    Ok(Json(sale))
}

async fn pending_outbox(
    State(st): State<AppState>,
) -> Result<Json<Vec<crate::db::OutboxRow>>, (StatusCode, String)> {
    st.db.list_pending_outbox(100).map(Json).map_err(err_500)
}

async fn ws(State(st): State<AppState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket: WebSocket| handle_ws(socket, st))
}

fn display_token(st: &AppState) -> Result<String, (StatusCode, String)> {
    if let Some(token) = st.db.get_setting("display_access_token").map_err(err_500)? {
        return Ok(token);
    }
    let token = uuid::Uuid::new_v4().simple().to_string();
    st.db
        .set_setting("display_access_token", &token)
        .map_err(err_500)?;
    Ok(token)
}

async fn tv_page(
    State(st): State<AppState>,
    Path(token): Path<String>,
) -> Result<Html<&'static str>, (StatusCode, String)> {
    if token != display_token(&st)? {
        return Err((StatusCode::NOT_FOUND, "TV não encontrada".into()));
    }
    Ok(Html(TV_HTML))
}

async fn tv_promotions(
    State(st): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<TvPayload>, (StatusCode, String)> {
    if token != display_token(&st)? {
        return Err((StatusCode::NOT_FOUND, "TV não encontrada".into()));
    }
    Ok(Json(TvPayload {
        settings: display_settings(&st)?,
        promotions: st.db.list_promotions(false).map_err(err_500)?,
    }))
}

/// Read-only display data on loopback so the local browser preview is identical to the TV.
async fn local_display_promotions(
    State(st): State<AppState>,
) -> Result<Json<Vec<crate::db::Promotion>>, (StatusCode, String)> {
    st.db.list_promotions(false).map(Json).map_err(err_500)
}

async fn local_display_settings(
    State(st): State<AppState>,
) -> Result<Json<DisplaySettings>, (StatusCode, String)> {
    display_settings(&st).map(Json)
}

async fn handle_ws(mut socket: WebSocket, st: AppState) {
    let mut rx = st.events.subscribe();
    while let Ok(payload) = rx.recv().await {
        if socket.send(Message::Text(payload.into())).await.is_err() {
            break;
        }
    }
}

fn err_500(e: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

pub async fn serve(state: AppState, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/products", get(list_products))
        .route("/api/products/search", get(search))
        .route("/api/products/{ean}", get(by_ean))
        .route("/api/products", axum::routing::post(upsert_product))
        .route("/api/outbox/pending", get(pending_outbox))
        .route("/api/sales", axum::routing::post(create_sale))
        .route("/api/sync/status", get(sync_status))
        .route("/api/display/promotions", get(local_display_promotions))
        .route("/api/display/settings", get(local_display_settings))
        .route("/ws", get(ws))
        .with_state(state)
        .layer(tower_http::cors::CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    log::info!("Servidor local Axum escutando em http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

/// A deliberately narrow LAN server. It never exposes checkout, stock or outbox routes.
pub async fn serve_display(state: AppState, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/tv/{token}", get(tv_page))
        .route("/api/tv/{token}/promotions", get(tv_promotions))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    log::info!("Servidor do Modo TV escutando em http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
