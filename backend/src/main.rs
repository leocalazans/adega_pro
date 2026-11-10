use std::{env, net::SocketAddr, sync::Arc};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{connect_info::ConnectInfo, FromRequestParts, Path, Query, State},
    http::{header, request::Parts, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::{get, patch, post, put},
    Json, Router,
};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, PgPool, Postgres, Transaction};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    db: PgPool,
}

// Rate limiting de login: máximo de 5 tentativas falhadas por email ou IP em 15 minutos.
const MAX_LOGIN_ATTEMPTS: i64 = 5;
const LOGIN_RATE_LIMIT_WINDOW_SECS: i64 = 15 * 60;

#[derive(Debug, sqlx::FromRow)]
struct LoginAttemptRecord {
    attempts: i64,
}

#[derive(Debug, Clone)]
struct TerminalAuth {
    tenant_id: Uuid,
    unit_id: Uuid,
    terminal_id: Uuid,
}

#[derive(Debug, Clone)]
struct OwnerAuth {
    tenant_id: Uuid,
}

/// Autenticação do painel do lojista: aceita a owner key (PDV desktop) ou a
/// sessão web do usuário do tenant. Mantém as duas superfícies no mesmo
/// conjunto de rotas e permite aplicar a regra de somente leitura.
#[derive(Debug, Clone)]
struct PanelAuth {
    tenant_id: Uuid,
}

#[derive(Debug, Clone)]
struct PlatformAuth {
    admin_id: Uuid,
    email: String,
}

#[derive(Debug, Clone)]
struct TenantAuth {
    user_id: Uuid,
    tenant_id: Uuid,
    email: String,
    role: String,
}

#[derive(Debug, Deserialize)]
struct PlatformLoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct TenantLoginRequest {
    email: String,
    password: String,
    /// Obrigatório apenas quando o mesmo e-mail existe em mais de uma loja.
    #[serde(default)]
    slug: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct TenantUserRecord {
    id: Uuid,
    tenant_id: Uuid,
    unit_id: Option<Uuid>,
    email: String,
    password_hash: String,
    role: String,
    display_name: Option<String>,
    must_change_password: bool,
}

#[derive(Debug, Deserialize)]
struct TenantPasswordRequest {
    current_password: String,
    new_password: String,
}

#[derive(Debug, sqlx::FromRow)]
struct PlatformAdminRecord {
    id: Uuid,
    email: String,
    password_hash: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct PlatformTenantOut {
    id: Uuid,
    name: String,
    slug: Option<String>,
    units: i64,
    terminals: i64,
    online_terminals: i64,
    subscription_status: Option<String>,
    billing_mode: Option<String>,
    plan: Option<String>,
    courtesy_until: Option<chrono::DateTime<chrono::Utc>>,
    current_period_ends_at: Option<chrono::DateTime<chrono::Utc>>,
    asaas_customer_id: Option<String>,
    owner_email: Option<String>,
    cnpj_cpf: Option<String>,
    phone: Option<String>,
    monthly_brl_cents: i64,
    mrr_brl_cents: i64,
    archived_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
struct PlatformTenantCreateRequest {
    name: String,
    unit_name: String,
    email: Option<String>,
    #[serde(default)]
    password: Option<String>,
    cnpj_cpf: Option<String>,
    phone: Option<String>,
    #[serde(default = "default_plan")]
    plan: String,
    #[serde(default)]
    trial_days: i64,
    create_asaas_customer: Option<bool>,
    #[serde(default)]
    billing_mode: Option<String>,
    courtesy_months: Option<i64>,
    courtesy_until: Option<chrono::DateTime<chrono::Utc>>,
    courtesy_note: Option<String>,
}
#[derive(Debug, Serialize)]
struct PlatformTenantCreated {
    tenant_id: Uuid,
    unit_id: Uuid,
    name: String,
    slug: String,
    trial_ends_at: chrono::DateTime<chrono::Utc>,
    asaas_customer_id: Option<String>,
    billing_mode: String,
    courtesy_until: Option<chrono::DateTime<chrono::Utc>>,
    owner_email: String,
    owner_password: String,
    owner_key: String,
}

#[derive(Debug, Deserialize)]
struct SignupRequest {
    name: String,
    unit_name: String,
    email: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    cnpj_cpf: Option<String>,
    #[serde(default)]
    phone: Option<String>,
    #[serde(default = "default_plan")]
    plan: String,
    #[serde(default)]
    billing_type: Option<String>,
    #[serde(default)]
    logo_data_url: Option<String>,
    #[serde(default)]
    primary_color: Option<String>,
}

#[derive(Debug, Serialize)]
struct SignupCreated {
    tenant_id: Uuid,
    unit_id: Uuid,
    name: String,
    slug: String,
    trial_ends_at: chrono::DateTime<chrono::Utc>,
    billing_mode: String,
    courtesy_until: Option<chrono::DateTime<chrono::Utc>>,
    owner_email: String,
    payment: Option<TenantPaymentIntentOut>,
    payment_error: Option<String>,
}

/// IP do cliente atrás do proxy do Render, com fallback para o socket.
fn client_ip(headers: &HeaderMap, addr: SocketAddr) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| addr.ip().to_string())
}
#[derive(Debug, Serialize, sqlx::FromRow)]
struct CommercialPlanOut {
    code: String,
    name: String,
    monthly_brl_cents: i32,
    description: String,
    fiscal_eligible: bool,
    premium_pdv: bool,
    active: bool,
}
#[derive(Debug, Serialize, sqlx::FromRow)]
struct EquipmentOut {
    id: Uuid,
    sku: String,
    name: String,
    category: String,
    sale_brl_cents: Option<i32>,
    rental_brl_cents: Option<i32>,
    source_url: Option<String>,
    active: bool,
}
#[derive(Debug, Deserialize)]
struct CommercialLeadRequest {
    store_name: String,
    contact_name: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    city: Option<String>,
    plan_code: Option<String>,
    #[serde(default = "default_modality")]
    modality: String,
    details: Option<String>,
}
fn default_modality() -> String {
    "purchase".into()
}
#[derive(Debug, Deserialize)]
struct EquipmentInput {
    sku: String,
    name: String,
    category: String,
    sale_brl_cents: Option<i32>,
    rental_brl_cents: Option<i32>,
    source_url: Option<String>,
    active: bool,
}
#[derive(Debug, Deserialize)]
struct PlatformActivationCodeRequest {
    #[serde(default = "default_activation_days")]
    valid_days: i64,
    #[serde(default = "default_activation_uses")]
    max_uses: i32,
}
#[derive(Debug, Serialize)]
struct PlatformActivationCodeCreated {
    id: Uuid,
    code: String,
    expires_at: chrono::DateTime<chrono::Utc>,
    max_uses: i32,
}
#[derive(Debug, Deserialize)]
struct ActivationClaimRequest {
    code: String,
    installation_id: String,
    terminal_name: String,
}
#[derive(Debug, Serialize)]
struct ActivationClaimed {
    tenant_id: Uuid,
    unit_id: Uuid,
    tenant_slug: String,
    tenant_name: String,
    terminal_id: Uuid,
    terminal_key: String,
    owner_key: String,
    license: SignedLicense,
}
fn default_plan() -> String {
    "premium".into()
}
fn default_activation_days() -> i64 {
    7
}
fn default_activation_uses() -> i32 {
    1
}

#[derive(Debug, Deserialize)]
struct CloudEventIn {
    uuid: Uuid,
    entity: String,
    operation: String,
    payload: Value,
    created_at: i64,
}

#[derive(Debug, Serialize)]
struct CloudEventOut {
    accepted: bool,
    duplicate: bool,
}

#[derive(Debug, Deserialize)]
struct SalePayload {
    uuid: Uuid,
    total_brl_cents: i64,
    payment_method: String,
    items: Vec<SaleItemPayload>,
    created_at: i64,
}

#[derive(Debug, Deserialize)]
struct SaleItemPayload {
    ean: String,
    qty: f64,
    price_brl_cents: i64,
}

#[derive(Debug, Deserialize)]
struct StockPayload {
    ean: String,
    delta: f64,
}

#[derive(Debug, Deserialize)]
struct ProductPayload {
    ean: String,
    part_number: String,
    description: String,
    brand: Option<String>,
    price_brl_cents: i64,
    min_stock: f64,
    #[serde(default = "default_true")]
    active: bool,
    #[serde(default)]
    image_url: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct ProductOut {
    ean: String,
    part_number: String,
    description: String,
    brand: Option<String>,
    price_brl_cents: i64,
    stock_qty: f64,
    min_stock: f64,
    active: bool,
    image_url: Option<String>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
struct ProductsQuery {
    limit: Option<i64>,
    since: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct SyncUsersQuery {
    since: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ReportsQuery {
    from: Option<i64>,
    to: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct UnitSalesOut {
    unit_id: Uuid,
    unit_name: String,
    sales_count: i64,
    total_brl_cents: i64,
    items_count: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct OwnerUnitOut {
    unit_id: Uuid,
    unit_name: String,
    unit_code: String,
    sales_count: i64,
    total_brl_cents: i64,
    items_count: i64,
    stock_skus: i64,
    low_stock_skus: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct OwnerTopProductOut {
    ean: String,
    description: String,
    qty_sold: f64,
    revenue_brl_cents: i64,
    units_sold_in: i64,
}

#[derive(Debug, Serialize)]
struct OwnerOverviewOut {
    total_brl_cents: i64,
    sales_count: i64,
    units: Vec<OwnerUnitOut>,
    top_products: Vec<OwnerTopProductOut>,
}

#[derive(Debug, Serialize)]
struct DashboardSummaryOut {
    sales_today_brl_cents: i64,
    items_today: i64,
    new_customers_today: i64,
    low_stock_count: i64,
    daily_sales: Vec<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct WebSalesRow {
    uuid: String,
    total_brl_cents: i64,
    payment_method: String,
    terminal_id: String,
    items_count: i64,
    created_at: i64,
}

#[derive(Debug, Deserialize)]
struct WebReportsQuery {
    #[serde(default)]
    from: Option<i64>,
    #[serde(default)]
    to: Option<i64>,
    #[serde(default)]
    payment_method: Option<String>,
    #[serde(default)]
    terminal_id: Option<String>,
    #[serde(default)]
    product_query: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UnitProductSettingRequest {
    unit_id: Uuid,
    ean: String,
    price_brl_cents: Option<i64>,
    min_stock: Option<f64>,
    active: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct OwnerPromotionUnitRequest {
    unit_id: Uuid,
    price_brl_cents: Option<i64>,
    #[serde(default = "default_true")]
    active: bool,
}

#[derive(Debug, Deserialize)]
struct OwnerPromotionRequest {
    id: Option<Uuid>,
    ean: Option<String>,
    title: String,
    subtitle: Option<String>,
    price_label: Option<String>,
    starts_at: Option<chrono::DateTime<chrono::Utc>>,
    ends_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default = "default_true")]
    active: bool,
    #[serde(default)]
    units: Vec<OwnerPromotionUnitRequest>,
}

#[derive(Debug, Deserialize)]
struct DisplayChannelRequest {
    unit_id: Uuid,
    name: String,
}

#[derive(Debug, Serialize)]
struct DisplayChannelCreated {
    id: Uuid,
    token: String,
    path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
struct TenantBranding {
    display_name: String,
    logo_data_url: Option<String>,
    primary_color: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TenantBrandingInput {
    #[serde(alias = "displayName")]
    display_name: String,
    #[serde(alias = "logoDataUrl")]
    logo_data_url: Option<String>,
    #[serde(alias = "primaryColor")]
    primary_color: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct DisplayPromotionOut {
    title: String,
    subtitle: Option<String>,
    price_label: String,
    image_url: Option<String>,
}

const ONLINE_TV_HTML: &str = r#"<!doctype html><html lang="pt-BR"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>CommerceCTRL TV</title><style>body{margin:0;background:#111426;color:white;font-family:Arial,sans-serif}main{min-height:100vh;display:grid;place-items:center;padding:5vw;box-sizing:border-box;text-align:center;background:radial-gradient(circle at 20% 20%,#5c66c2,#202442 60%,#111426)}small{letter-spacing:.2em;color:#cfc1ff}h1{font-size:clamp(3rem,9vw,9rem);margin:.25em 0;text-transform:uppercase}.sub{font-size:clamp(1.5rem,3vw,3rem);color:#f4cc52}.price{font-size:clamp(4rem,12vw,12rem);font-weight:900;color:#ffe476;margin:.25em 0}.muted{color:#b8bddf}</style><main><article><small>OFERTAS DA LOJA</small><h1 id="title">Carregando ofertas</h1><p class="sub" id="subtitle"></p><p class="price" id="price"></p><p class="muted" id="status">Atualização automática ativa</p></article></main><script>const token=location.pathname.split('/').pop(),endpoint='/api/v1/display/'+encodeURIComponent(token)+'/promotions',title=document.getElementById('title'),subtitle=document.getElementById('subtitle'),price=document.getElementById('price'),status=document.getElementById('status');let list=[],i=0;function render(){const p=list[i%list.length];if(!p){title.textContent='Sem promoções ativas';subtitle.textContent='';price.textContent='';return}title.textContent=p.title;subtitle.textContent=p.subtitle||'OFERTA';price.textContent=p.price_label;i=(i+1)%list.length}async function load(){try{const r=await fetch(endpoint,{cache:'no-store'});if(!r.ok)throw Error();const next=await r.json();if(JSON.stringify(next)!==JSON.stringify(list)){list=next;i=0;render()}status.textContent='Atualizado '+new Date().toLocaleTimeString('pt-BR')}catch(e){status.textContent='Sem conexão com o servidor'}}load();setInterval(load,5000);setInterval(render,10000);</script>"#;

const ONLINE_TV_FOLHETO_HTML: &str = r#"<!doctype html><html lang="pt-BR"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><style>body{margin:0;min-height:100vh;background:#f7e9bb;color:#21180e;font-family:Arial;font-weight:700}main{min-height:100vh;padding:3vw;box-sizing:border-box;display:grid;grid-template-rows:auto 1fr auto;background:repeating-linear-gradient(-25deg,transparent 0 34px,#f4df9e 35px 37px)}header{background:#d71920;color:#fff;border:8px solid #fff;padding:2vw 3vw;box-shadow:0 8px #a20d12;display:flex;justify-content:space-between;align-items:center}header small{letter-spacing:.2em}header strong{font-size:clamp(1.4rem,4vw,4rem);font-style:italic}.offer{align-self:center;display:grid;grid-template-columns:1fr .7fr;align-items:center;gap:3vw;transition:.45s}.out{opacity:0;transform:scale(.97)}.tag{display:inline-block;background:#ffcf18;padding:.4em .8em;box-shadow:4px 4px #21180e;font-size:clamp(1rem,2vw,2.6rem)}h1{font-size:clamp(3rem,9vw,10rem);line-height:.85;margin:.3em 0;text-transform:uppercase;letter-spacing:-.07em;text-shadow:4px 4px #fff}.sub{font-size:clamp(1rem,2.3vw,2.7rem)}.price{background:#d71920;border:8px solid #fff;border-radius:50%;width:min(34vw,460px);aspect-ratio:1;display:grid;place-items:center;color:#fff;text-align:center;font-size:clamp(3rem,8vw,9rem);box-shadow:0 0 0 8px #d71920;transform:rotate(4deg)}.price small{font-size:.22em;text-transform:uppercase}footer{border-top:3px solid;padding-top:1vw;display:flex;justify-content:space-between;color:#d71920}@media(max-width:700px){.offer{grid-template-columns:1fr}.price{width:60vw;margin:auto}}</style><main><header><small>OFERTAS IMPERDÍVEIS</small><strong>Mercado Bom Vizinho</strong></header><section class="offer" id="o"><div><span class="tag">OFERTA DA VEZ</span><h1 id="t">Carregando</h1><p class="sub" id="s"></p></div><div class="price" id="p">R$<small>preço especial</small></div></section><footer><span>Válido enquanto durarem os estoques</span><span id="x">Atualizando...</span></footer></main><script>const k=location.pathname.split('/').pop(),e='/api/v1/display/'+encodeURIComponent(k)+'/promotions',o=document.querySelector('#o'),t=document.querySelector('#t'),s=document.querySelector('#s'),p=document.querySelector('#p'),x=document.querySelector('#x');let a=[],i=0;function r(){let q=a[i%a.length];o.classList.add('out');setTimeout(()=>{t.textContent=q?q.title:'SEM OFERTAS';s.textContent=q?(q.subtitle||'Preço especial'):'Cadastre promoções no painel';p.innerHTML=q?(q.price_label||'CONSULTE')+'<small>preço especial</small>':'—';o.classList.remove('out');i=(i+1)%a.length},350)}async function l(){try{let z=await fetch(e,{cache:'no-store'});a=await z.json();if(a.length)r();x.textContent='Ofertas atualizadas'}catch(_){x.textContent='Sem conexão'}}l();setInterval(l,5000);setInterval(()=>a.length>1&&r(),10000)</script>"#;

const ONLINE_TV_FOLHETO_COM_IMAGEM_HTML: &str = r#"<!doctype html><html lang='pt-BR'><meta charset='utf-8'><meta name='viewport' content='width=device-width,initial-scale=1'><style>body{margin:0;background:#f7e9bb;color:#21180e;font-family:Arial;font-weight:800}main{min-height:100vh;padding:3vw;box-sizing:border-box;display:grid;grid-template-rows:auto 1fr auto;background:repeating-linear-gradient(-25deg,transparent 0 34px,#f4df9e 35px 37px)}header{background:#d71920;color:#fff;border:8px solid #fff;padding:2vw 3vw;box-shadow:0 8px #a20d12;display:flex;justify-content:space-between}header strong{font-size:clamp(1.4rem,4vw,4rem);font-style:italic}.offer{align-self:center;display:grid;grid-template-columns:1fr .7fr;gap:3vw;align-items:center;transition:.45s}.out{opacity:0;transform:scale(.97)}.tag{background:#ffcf18;padding:.4em .8em;box-shadow:4px 4px #21180e;font-size:clamp(1rem,2vw,2.6rem)}h1{font-size:clamp(3rem,8vw,9rem);line-height:.85;margin:.3em 0;text-transform:uppercase;text-shadow:4px 4px #fff}.sub{font-size:clamp(1rem,2.3vw,2.7rem)}.visual{background:#fff;border:8px solid #d71920;border-radius:28px;padding:2vw;box-shadow:10px 12px #b48821;display:grid;place-items:center}.visual img{width:min(28vw,390px);height:min(28vw,390px);object-fit:contain}.price{background:#d71920;color:white;border:6px solid white;border-radius:50%;padding:.3em .5em;font-size:clamp(2.4rem,6vw,7rem);margin-top:-5vw;z-index:2}footer{border-top:3px solid;padding-top:1vw;display:flex;justify-content:space-between;color:#d71920}@media(max-width:700px){.offer{grid-template-columns:1fr}.visual img{width:55vw;height:45vw}}</style><main><header><small>OFERTAS IMPERDÍVEIS</small><strong>Mercado Bom Vizinho</strong></header><section class='offer' id='o'><div><span class='tag'>OFERTA DA VEZ</span><h1 id='t'>Carregando</h1><p class='sub' id='s'></p></div><div class='visual'><img id='img' alt='Produto em promoção'><div class='price' id='p'>R$</div></div></section><footer><span>Válido enquanto durarem os estoques</span><span id='x'>Atualizando...</span></footer></main><script>const k=location.pathname.split('/').pop(),e='/api/v1/display/'+encodeURIComponent(k)+'/promotions',o=document.querySelector('#o'),t=document.querySelector('#t'),s=document.querySelector('#s'),p=document.querySelector('#p'),img=document.querySelector('#img'),x=document.querySelector('#x'),pics={'Cerveja Pilsen Pack 6':'https://www.savegnagoio.vtexassets.com/arquivos/ids/380844/CervejaLagunitasIPA350mlLata4.jpg?v=638104382866400000','Cerveja Artesanal IPA':'https://www.centralcervejas.pt/media/1qjfrpog/lagunitas-ipa.png?anchor=center&preset=textimagebigmin320&rnd=132844879593970000','Energético Power':'https://mendozastore.it/cdn/shop/products/energ_1024x1024.jpg?v=1670198584'};let a=[],i=0;function r(){let q=a[i%a.length];o.classList.add('out');setTimeout(()=>{t.textContent=q?q.title:'SEM OFERTAS';s.textContent=q?(q.subtitle||'Preço especial'):'Cadastre promoções';p.textContent=q?(q.price_label||'CONSULTE'):'—';img.src=q?(pics[q.title]||pics['Energético Power']):'';o.classList.remove('out');i=(i+1)%a.length},350)}async function l(){try{let z=await fetch(e,{cache:'no-store'});a=await z.json();if(a.length)r();x.textContent='Ofertas atualizadas'}catch(_){x.textContent='Sem conexão'}}l();setInterval(l,5000);setInterval(()=>a.length>1&&r(),10000)</script>"#;

const ONLINE_TV_DEMO_HTML: &str = r#"<!doctype html><html lang='pt-BR'><meta charset='utf-8'><meta name='viewport' content='width=device-width,initial-scale=1'><style>*{box-sizing:border-box}body{margin:0;background:#ffd928;color:#1c160b;font:800 16px Arial}main{min-height:100vh;padding:3vw;display:grid;grid-template-rows:auto 1fr auto;gap:2vw}header{background:#d71920;color:#fff;border:8px solid #fff;padding:1.4vw 2.5vw;box-shadow:0 8px #9d1015;display:flex;justify-content:space-between;align-items:center}header strong{font-size:clamp(1.7rem,4vw,4.5rem);font-style:italic;text-transform:uppercase}.offer{align-self:center;display:grid;grid-template-columns:1.15fr .85fr;gap:3vw;align-items:center;transition:.45s}.offer.out{opacity:0;transform:scale(.97)}.tag{display:inline-block;background:#fff;padding:.45em .8em;box-shadow:5px 5px #1c160b;font-size:clamp(1rem,2.2vw,2.8rem)}h1{font-size:clamp(3rem,8vw,9rem);line-height:.86;margin:.3em 0;text-transform:uppercase;letter-spacing:-.06em}.sub{font-size:clamp(1rem,2.2vw,2.7rem)}.visual{position:relative;min-height:min(42vw,520px);background:#fff;border:8px solid #d71920;border-radius:28px;box-shadow:12px 14px #a98512;overflow:hidden}.sprite{position:absolute;inset:3%;background-image:url('/assets/demo-products-v1.png');background-repeat:no-repeat;background-size:400% 200%;background-position:var(--x) var(--y)}.price{position:absolute;right:2%;bottom:2%;background:#d71920;color:#fff;border:6px solid #fff;border-radius:50%;padding:.42em .55em;font-size:clamp(2.2rem,5vw,6rem);z-index:2}footer{display:flex;justify-content:space-between;border-top:3px solid;padding-top:1vw;color:#9d1015}@media(max-width:700px){.offer{grid-template-columns:1fr}.visual{min-height:50vh}}</style><main><header><small>OFERTAS IMPERDÍVEIS</small><strong>Mercadinho Martins</strong></header><section class='offer' id='o'><div><span class='tag'>OFERTA DA VEZ</span><h1 id='t'>Carregando</h1><p class='sub' id='s'></p></div><div class='visual'><div class='sprite' id='img'></div><div class='price' id='p'>R$</div></div></section><footer><span>Válido enquanto durarem os estoques</span><span id='x'>Atualizando...</span></footer></main><script>const k=location.pathname.split('/').pop(),e='/api/v1/display/'+encodeURIComponent(k)+'/promotions',o=document.querySelector('#o'),t=document.querySelector('#t'),s=document.querySelector('#s'),p=document.querySelector('#p'),img=document.querySelector('#img'),x=document.querySelector('#x'),pos={'Vinho Tinto Suave':[0,0],'Cerveja Artesanal IPA':[33.333,0],'Whisky 12 Anos':[66.666,0],'Gin Importado':[100,0],'Água Tônica':[0,100],'Energético Power':[33.333,100],'Saca-rolhas':[66.666,100],'Cerveja Pilsen Pack 6':[100,100]};let a=[],i=0;function r(){let q=a[i%a.length],v=q?(pos[q.title]||[33.333,100]):[0,0];o.classList.add('out');setTimeout(()=>{t.textContent=q?q.title:'SEM OFERTAS';s.textContent=q?(q.subtitle||'Preço especial'):'Cadastre promoções';p.textContent=q?(q.price_label||'CONSULTE'):'—';img.style.setProperty('--x',v[0]+'%');img.style.setProperty('--y',v[1]+'%');o.classList.remove('out');i=(i+1)%a.length},350)}async function l(){try{let z=await fetch(e,{cache:'no-store'});a=await z.json();if(a.length)r();x.textContent='Ofertas atualizadas'}catch(_){x.textContent='Sem conexão'}}l();setInterval(l,5000);setInterval(()=>a.length>1&&r(),10000)</script>"#;

const ONLINE_TV_CATALOG_HTML: &str = r#"<!doctype html><html lang='pt-BR'><meta charset='utf-8'><meta name='viewport' content='width=device-width,initial-scale=1'><title>Ofertas</title><style>*{box-sizing:border-box}body{margin:0;background:#f6e8ae;color:#20180d;font:800 16px Arial}main{min-height:100vh;padding:3vw;display:grid;grid-template-rows:auto 1fr auto;gap:2vw;background:repeating-linear-gradient(-24deg,transparent 0 32px,#efd994 33px 35px)}header{background:#d71920;color:white;border:7px solid white;padding:1.4vw 2.5vw;box-shadow:0 8px #981017;display:flex;justify-content:space-between;align-items:center}header strong{font-size:clamp(1.5rem,4vw,4.2rem);font-style:italic;text-transform:uppercase}.offer{align-self:center;display:grid;grid-template-columns:1.1fr .9fr;gap:3vw;align-items:center;transition:opacity .35s,transform .35s}.out{opacity:0;transform:scale(.97)}.tag{display:inline-block;background:#ffcf18;padding:.42em .8em;box-shadow:4px 4px #20180d;font-size:clamp(1rem,2vw,2.5rem)}h1{font-size:clamp(3rem,8vw,9rem);line-height:.85;margin:.3em 0;text-transform:uppercase;letter-spacing:-.07em;text-shadow:4px 4px white}.sub{font-size:clamp(1rem,2.2vw,2.5rem)}.visual{position:relative;min-height:min(42vw,510px);background:#fff;border:8px solid #d71920;border-radius:28px;box-shadow:12px 14px #b28a25;overflow:hidden;display:grid;place-items:center}.visual img{width:100%;height:100%;object-fit:contain;padding:3vw}.fallback{font-size:clamp(5rem,15vw,12rem);color:#d71920}.price{position:absolute;right:3%;bottom:3%;background:#d71920;color:white;border:6px solid white;border-radius:50%;padding:.4em .55em;font-size:clamp(2.2rem,5vw,6rem)}footer{display:flex;justify-content:space-between;border-top:3px solid;padding-top:1vw;color:#a00d14}@media(max-width:700px){.offer{grid-template-columns:1fr}.visual{min-height:45vh}}</style><main><header><small>OFERTAS IMPERDÍVEIS</small><strong id='store'>Sua loja</strong></header><section class='offer' id='offer'><div><span class='tag'>OFERTA DA VEZ</span><h1 id='title'>Carregando</h1><p class='sub' id='subtitle'></p></div><div class='visual'><img id='image' alt='Produto em promoção'><span class='fallback' id='fallback'>%</span><div class='price' id='price'>R$</div></div></section><footer><span>Válido enquanto durarem os estoques</span><span id='status'>Atualizando…</span></footer></main><script>const token=location.pathname.split('/').pop(),endpoint='/api/v1/display/'+encodeURIComponent(token)+'/promotions',o=document.querySelector('#offer'),t=document.querySelector('#title'),s=document.querySelector('#subtitle'),p=document.querySelector('#price'),im=document.querySelector('#image'),f=document.querySelector('#fallback'),x=document.querySelector('#status');let a=[],i=0;function r(){let q=a[i%a.length];o.classList.add('out');setTimeout(()=>{t.textContent=q?q.title:'SEM OFERTAS';s.textContent=q?(q.subtitle||'Preço especial'):'Cadastre promoções no painel';p.textContent=q?(q.price_label||'CONSULTE'):'—';im.src=q&&q.image_url?q.image_url:'';im.style.display=q&&q.image_url?'block':'none';f.style.display=q&&q.image_url?'none':'block';o.classList.remove('out');i=(i+1)%Math.max(a.length,1)},320)}async function l(){try{let z=await fetch(endpoint,{cache:'no-store'});a=await z.json();r();x.textContent='Ofertas atualizadas'}catch(_){x.textContent='Sem conexão'}}l();setInterval(l,5000);setInterval(()=>a.length>1&&r(),10000)</script>"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LicenseClaims {
    tenant_id: String,
    unit_id: String,
    installation_id: String,
    issued_at: i64,
    expires_at: i64,
    grace_until: i64,
    nonce: String,
}

#[derive(Debug, Serialize)]
struct SignedLicense {
    claims: LicenseClaims,
    signature: String,
}

#[derive(Debug, Deserialize)]
struct LicenseIssueRequest {
    tenant_id: Uuid,
    unit_id: Uuid,
    installation_id: String,
    #[serde(default = "default_valid_days")]
    valid_days: i64,
    #[serde(default = "default_grace_days")]
    grace_days: i64,
}

#[derive(Debug, Deserialize)]
struct LicenseRenewRequest {
    installation_id: String,
}

#[derive(Debug, Deserialize)]
struct PixIntentRequest {
    external_reference: String,
    amount_brl_cents: i64,
    description: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct PaymentIntentOut {
    id: Uuid,
    provider: String,
    provider_id: Option<String>,
    external_reference: String,
    amount_brl_cents: i64,
    status: String,
    qr_payload: Option<String>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
struct AsaasPaymentCreated {
    id: String,
    status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AsaasPixQr {
    payload: String,
    expiration_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AsaasWebhook {
    event: String,
    payment: Value,
}

fn asaas_status(value: &str) -> &'static str {
    match value {
        "RECEIVED" | "CONFIRMED" => "received",
        "REFUNDED" | "REFUND_REQUESTED" => "refunded",
        "CHARGEBACK" | "CHARGEBACK_REQUESTED" | "CHARGEBACK_DISPUTE" => "chargeback",
        "OVERDUE" => "overdue",
        "DELETED" => "cancelled",
        _ => "pending",
    }
}

async fn create_pix_intent(
    State(state): State<Arc<AppState>>,
    auth: TerminalAuth,
    Json(request): Json<PixIntentRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if request.amount_brl_cents <= 0
        || request.external_reference.trim().is_empty()
        || request.external_reference.len() > 100
    {
        return Err(error(StatusCode::BAD_REQUEST, "cobrança PIX inválida"));
    }
    if let Some(existing) = sqlx::query_as::<_, PaymentIntentOut>("SELECT id,provider,provider_id,external_reference,amount_brl_cents,status,qr_payload,expires_at FROM payment_intents WHERE tenant_id=$1 AND external_reference=$2")
        .bind(auth.tenant_id).bind(request.external_reference.trim()).fetch_optional(&state.db).await.map_err(internal)? {
        if existing.amount_brl_cents != request.amount_brl_cents { return Err(error(StatusCode::CONFLICT, "referência já usada com outro valor")); }
        return Ok(Json(existing));
    }
    let api_key = env::var("ASAAS_API_KEY").unwrap_or_default();
    let customer = env::var("ASAAS_CUSTOMER_ID").unwrap_or_default();
    if api_key.is_empty() || customer.is_empty() {
        return Err(error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Asaas não configurado",
        ));
    }
    let base = env::var("ASAAS_BASE_URL")
        .unwrap_or_else(|_| "https://api-sandbox.asaas.com/v3".into())
        .trim_end_matches('/')
        .to_string();
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO payment_intents(id,tenant_id,unit_id,terminal_id,provider,external_reference,amount_brl_cents,status) VALUES($1,$2,$3,$4,'asaas',$5,$6,'creating')")
        .bind(id).bind(auth.tenant_id).bind(auth.unit_id).bind(auth.terminal_id).bind(request.external_reference.trim()).bind(request.amount_brl_cents).execute(&state.db).await.map_err(internal)?;
    let client = reqwest::Client::new();
    let created_response = client
        .post(format!("{base}/payments"))
        .header("access_token", &api_key)
        .json(&serde_json::json!({
            "customer": customer,
            "billingType": "PIX",
            "value": request.amount_brl_cents as f64 / 100.0,
            "dueDate": chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string(),
            "description": request.description.unwrap_or_else(|| "Venda CommerceCTRL".into()),
            "externalReference": request.external_reference.trim(),
        }))
        .send()
        .await
        .map_err(|e| error(StatusCode::BAD_GATEWAY, format!("Asaas indisponível: {e}")))?;
    if !created_response.status().is_success() {
        let status = created_response.status();
        let body = created_response.text().await.unwrap_or_default();
        sqlx::query(
            "UPDATE payment_intents SET status='failed',raw=$2,updated_at=now() WHERE id=$1",
        )
        .bind(id)
        .bind(serde_json::json!({"http_status":status.as_u16(),"body":body}))
        .execute(&state.db)
        .await
        .map_err(internal)?;
        return Err(error(
            StatusCode::BAD_GATEWAY,
            "Asaas recusou a criação da cobrança",
        ));
    }
    let created: AsaasPaymentCreated = created_response.json().await.map_err(internal)?;
    let qr_response = client
        .get(format!("{base}/payments/{}/pixQrCode", created.id))
        .header("access_token", &api_key)
        .send()
        .await
        .map_err(|e| error(StatusCode::BAD_GATEWAY, format!("QR PIX indisponível: {e}")))?;
    if !qr_response.status().is_success() {
        return Err(error(
            StatusCode::BAD_GATEWAY,
            "Asaas não retornou o QR PIX",
        ));
    }
    let qr: AsaasPixQr = qr_response.json().await.map_err(internal)?;
    let expires_at = qr
        .expiration_date
        .as_deref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&chrono::Utc));
    sqlx::query("UPDATE payment_intents SET provider_id=$2,status=$3,qr_payload=$4,expires_at=$5,raw=$6,updated_at=now() WHERE id=$1")
        .bind(id).bind(&created.id).bind(asaas_status(&created.status)).bind(&qr.payload).bind(expires_at).bind(serde_json::json!({"asaas_status":created.status})).execute(&state.db).await.map_err(internal)?;
    let result = sqlx::query_as::<_, PaymentIntentOut>("SELECT id,provider,provider_id,external_reference,amount_brl_cents,status,qr_payload,expires_at FROM payment_intents WHERE id=$1").bind(id).fetch_one(&state.db).await.map_err(internal)?;
    Ok(Json(result))
}

async fn payment_intent(
    State(state): State<Arc<AppState>>,
    auth: TerminalAuth,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let result = sqlx::query_as::<_, PaymentIntentOut>("SELECT id,provider,provider_id,external_reference,amount_brl_cents,status,qr_payload,expires_at FROM payment_intents WHERE id=$1 AND tenant_id=$2 AND unit_id=$3")
        .bind(id).bind(auth.tenant_id).bind(auth.unit_id).fetch_optional(&state.db).await.map_err(internal)?
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "cobrança não encontrada"))?;
    Ok(Json(result))
}

async fn asaas_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<AsaasWebhook>,
) -> Result<StatusCode, ApiError> {
    let expected = env::var("ASAAS_WEBHOOK_TOKEN").unwrap_or_default();
    let provided = headers
        .get("asaas-access-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if expected.len() < 24 || hash_key(provided) != hash_key(&expected) {
        return Err(error(StatusCode::UNAUTHORIZED, "webhook não autorizado"));
    }
    let provider_id = payload
        .payment
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| error(StatusCode::BAD_REQUEST, "pagamento sem id"))?;
    let remote_status = payload
        .payment
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("");
    let normalized = asaas_status(remote_status);
    sqlx::query("UPDATE payment_intents SET status=$2,raw=raw||$3,updated_at=now() WHERE provider='asaas' AND provider_id=$1")
        .bind(provider_id)
        .bind(normalized)
        .bind(serde_json::json!({"event":payload.event,"payment":payload.payment}))
        .execute(&state.db)
        .await
        .map_err(internal)?;

    let intent = sqlx::query_as::<_, (Uuid, Option<i32>)>(
        "SELECT tenant_id, NULLIF(raw->>'validity_days','')::int FROM payment_intents WHERE provider='asaas' AND provider_id=$1",
    )
    .bind(provider_id)
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?;
    if let Some((tenant_id, validity_days)) = intent {
        let days = validity_days.unwrap_or(30).clamp(1, 730);
        match normalized {
            "received" | "confirmed" => {
                let ends_at = sqlx::query_scalar::<_, chrono::DateTime<chrono::Utc>>(
                    "UPDATE tenant_subscriptions SET status='active',billing_mode='paid',courtesy_until=NULL,courtesy_note=NULL,trial_ends_at=NULL,current_period_ends_at=GREATEST(coalesce(current_period_ends_at,now()),now())+($2::text)::interval,updated_at=now() WHERE tenant_id=$1 RETURNING current_period_ends_at",
                )
                .bind(tenant_id)
                .bind(format!("{days} days"))
                .fetch_optional(&state.db)
                .await
                .map_err(internal)?;
                if let Some(ends_at) = ends_at {
                    let expires = ends_at.timestamp();
                    sqlx::query("UPDATE licenses SET status='active',expires_at=$1,grace_until=$2,updated_at=now() WHERE tenant_id=$3")
                        .bind(expires)
                        .bind(expires + 7 * 86_400)
                        .bind(tenant_id)
                        .execute(&state.db)
                        .await
                        .map_err(internal)?;
                }
                info!(tenant=%tenant_id, days, "assinatura liberada pelo webhook Asaas");
            }
            "overdue" => {
                sqlx::query("UPDATE tenant_subscriptions SET status='past_due',updated_at=now() WHERE tenant_id=$1 AND billing_mode='paid' AND status<>'cancelled'")
                    .bind(tenant_id)
                    .execute(&state.db)
                    .await
                    .map_err(internal)?;
            }
            "refunded" | "chargeback" => {
                sqlx::query("UPDATE tenant_subscriptions SET status='suspended',updated_at=now() WHERE tenant_id=$1")
                    .bind(tenant_id)
                    .execute(&state.db)
                    .await
                    .map_err(internal)?;
                sqlx::query(
                    "UPDATE licenses SET status='suspended',updated_at=now() WHERE tenant_id=$1",
                )
                .bind(tenant_id)
                .execute(&state.db)
                .await
                .map_err(internal)?;
            }
            _ => {}
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

fn default_valid_days() -> i64 {
    30
}
fn default_grace_days() -> i64 {
    15
}

fn signing_key() -> Result<SigningKey, ApiError> {
    if let Ok(raw) = env::var("LICENSE_SIGNING_KEY_B64") {
        let decoded = STANDARD
            .decode(raw.trim())
            .map_err(|_| error(StatusCode::SERVICE_UNAVAILABLE, "chave de licença inválida"))?;
        let bytes: [u8; 32] = decoded.try_into().map_err(|_| {
            error(
                StatusCode::SERVICE_UNAVAILABLE,
                "chave de licença deve ter 32 bytes",
            )
        })?;
        return Ok(SigningKey::from_bytes(&bytes));
    }
    if cfg!(debug_assertions) {
        return Ok(SigningKey::from_bytes(&[7_u8; 32]));
    }
    Err(error(
        StatusCode::SERVICE_UNAVAILABLE,
        "emissor de licença não configurado",
    ))
}

fn sign_license(claims: LicenseClaims) -> Result<SignedLicense, ApiError> {
    let payload = serde_json::to_vec(&claims).map_err(internal)?;
    let signature = signing_key()?.sign(&payload);
    Ok(SignedLicense {
        claims,
        signature: STANDARD.encode(signature.to_bytes()),
    })
}

// A chave verificadora é pública por definição: o PDV precisa incorporá-la para
// validar licenças offline. Expor somente este valor permite que o processo de
// release use exatamente a chave do backend sem revelar a chave de assinatura.
async fn license_public_key() -> Result<Json<Value>, ApiError> {
    use base64::Engine;
    let public_key_b64 =
        base64::engine::general_purpose::STANDARD.encode(signing_key()?.verifying_key().to_bytes());
    Ok(Json(
        serde_json::json!({ "public_key_b64": public_key_b64 }),
    ))
}

fn require_license_admin(headers: &HeaderMap) -> Result<(), ApiError> {
    let expected = env::var("LICENSE_ADMIN_KEY").unwrap_or_else(|_| {
        if cfg!(debug_assertions) {
            "commercectrl-license-dev".into()
        } else {
            String::new()
        }
    });
    let provided = headers
        .get("x-license-admin-key")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if expected.len() < 24 || hash_key(provided) != hash_key(&expected) {
        return Err(error(
            StatusCode::UNAUTHORIZED,
            "administrador de licença não autorizado",
        ));
    }
    Ok(())
}

async fn issue_license(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<LicenseIssueRequest>,
) -> Result<impl IntoResponse, ApiError> {
    require_license_admin(&headers)?;
    if request.installation_id.trim().is_empty()
        || !(1..=365).contains(&request.valid_days)
        || !(0..=90).contains(&request.grace_days)
    {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "parâmetros da licença inválidos",
        ));
    }
    let now = chrono::Utc::now().timestamp();
    let expires_at = now + request.valid_days * 86_400;
    let grace_until = expires_at + request.grace_days * 86_400;
    sqlx::query("INSERT INTO licenses(tenant_id,unit_id,installation_id,status,expires_at,grace_until) VALUES($1,$2,$3,'active',$4,$5) ON CONFLICT(tenant_id,unit_id,installation_id) DO UPDATE SET status='active',expires_at=excluded.expires_at,grace_until=excluded.grace_until,updated_at=now()")
        .bind(request.tenant_id).bind(request.unit_id).bind(request.installation_id.trim()).bind(expires_at).bind(grace_until).execute(&state.db).await.map_err(internal)?;
    sqlx::query("INSERT INTO license_audit(tenant_id,unit_id,installation_id,action,detail) VALUES($1,$2,$3,'issued',$4)")
        .bind(request.tenant_id).bind(request.unit_id).bind(request.installation_id.trim()).bind(serde_json::json!({"expires_at":expires_at,"grace_until":grace_until})).execute(&state.db).await.map_err(internal)?;
    let claims = LicenseClaims {
        tenant_id: request.tenant_id.to_string(),
        unit_id: request.unit_id.to_string(),
        installation_id: request.installation_id,
        issued_at: now,
        expires_at,
        grace_until,
        nonce: Uuid::new_v4().to_string(),
    };
    Ok(Json(sign_license(claims)?))
}

async fn renew_license(
    State(state): State<Arc<AppState>>,
    auth: TerminalAuth,
    Json(request): Json<LicenseRenewRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let row = sqlx::query_as::<_, (String, i64, i64)>("SELECT status,expires_at,grace_until FROM licenses WHERE tenant_id=$1 AND unit_id=$2 AND installation_id=$3")
        .bind(auth.tenant_id).bind(auth.unit_id).bind(request.installation_id.trim()).fetch_optional(&state.db).await.map_err(internal)?
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "licença não cadastrada"))?;
    if row.0 != "active" {
        return Err(error(StatusCode::PAYMENT_REQUIRED, "licença suspensa"));
    }
    let now = chrono::Utc::now();
    // Recalcula o prazo a partir da assinatura: cortesia estendida ou renovação
    // chegam ao terminal no próximo pedido de licença.
    let subscription = sqlx::query_as::<_, (Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT current_period_ends_at,trial_ends_at FROM tenant_subscriptions WHERE tenant_id=$1 AND status IN ('trial','active') AND (current_period_ends_at IS NULL OR current_period_ends_at>now())",
    )
    .bind(auth.tenant_id)
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?;
    let subscription_until =
        subscription.and_then(|(period, trial)| trial.filter(|value| *value > now).or(period));
    let (expires_at, grace_until) = match subscription_until {
        Some(until) => {
            let expires = until.timestamp();
            let grace = expires + 7 * 86_400;
            if expires > row.1 || grace > row.2 {
                sqlx::query("UPDATE licenses SET expires_at=$1,grace_until=$2,updated_at=now() WHERE tenant_id=$3 AND unit_id=$4 AND installation_id=$5")
                    .bind(expires)
                    .bind(grace)
                    .bind(auth.tenant_id)
                    .bind(auth.unit_id)
                    .bind(request.installation_id.trim())
                    .execute(&state.db)
                    .await
                    .map_err(internal)?;
            }
            (expires.max(row.1), grace.max(row.2))
        }
        None => {
            if row.2 < now.timestamp() {
                return Err(error(StatusCode::PAYMENT_REQUIRED, "assinatura vencida"));
            }
            (row.1, row.2)
        }
    };
    let claims = LicenseClaims {
        tenant_id: auth.tenant_id.to_string(),
        unit_id: auth.unit_id.to_string(),
        installation_id: request.installation_id,
        issued_at: now.timestamp(),
        expires_at,
        grace_until,
        nonce: Uuid::new_v4().to_string(),
    };
    Ok(Json(sign_license(claims)?))
}

type ApiError = (StatusCode, Json<Value>);

fn error(status: StatusCode, message: impl Into<String>) -> ApiError {
    (status, Json(serde_json::json!({ "error": message.into() })))
}

/// Transforma o nome da loja em um slug estável para login/disambiguacão.
fn slugify(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut last_dash = false;
    for ch in name.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_dash = false;
        } else if !last_dash && !slug.is_empty() {
            slug.push('-');
            last_dash = true;
        }
    }
    let trimmed = slug.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "loja".into()
    } else {
        trimmed.chars().take(48).collect()
    }
}

fn random_secret(prefix: &str) -> String {
    format!(
        "{prefix}{}{}",
        Uuid::new_v4().simple(),
        &Uuid::new_v4().simple().to_string()[..12]
    )
}

fn temporary_password() -> String {
    // 23 caracteres, acima do mínimo de 12 exigido pelo bootstrap.
    format!("Cc!{}", &Uuid::new_v4().simple().to_string()[..20])
}

async fn unique_slug(db: &PgPool, name: &str) -> Result<String, sqlx::Error> {
    let base = slugify(name);
    let mut candidate = base.clone();
    let mut attempt = 1_i32;
    loop {
        let taken =
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM tenants WHERE slug=$1)")
                .bind(&candidate)
                .fetch_one(db)
                .await?;
        if !taken {
            return Ok(candidate);
        }
        attempt += 1;
        candidate = format!("{base}-{attempt}");
    }
}

fn header_uuid(headers: &HeaderMap, name: &'static str) -> Result<Uuid, ApiError> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| error(StatusCode::UNAUTHORIZED, format!("header {name} inválido")))
}

fn hash_key(key: &str) -> String {
    hex::encode(Sha256::digest(key.as_bytes()))
}

fn password_hash(value: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(value.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(internal)
}

async fn platform_audit(
    pool: &PgPool,
    admin_id: Uuid,
    action: &str,
    tenant_id: Option<Uuid>,
    metadata: Value,
) {
    if let Err(cause) = sqlx::query(
        "INSERT INTO platform_audit_events(admin_id,action,tenant_id,metadata) VALUES($1,$2,$3,$4)",
    )
    .bind(admin_id)
    .bind(action)
    .bind(tenant_id)
    .bind(metadata)
    .execute(pool)
    .await
    {
        warn!(?cause, "falha ao gravar auditoria da plataforma");
    }
}

fn platform_session_cookie(token: &str) -> Result<HeaderValue, ApiError> {
    format!(
        "__Host-commercectrl_session={token}; Path=/; HttpOnly; Secure; SameSite=None; Max-Age=604800"
    )
    .parse()
    .map_err(internal)
}

fn expired_platform_session_cookie() -> HeaderValue {
    "__Host-commercectrl_session=; Path=/; HttpOnly; Secure; SameSite=None; Max-Age=0"
        .parse()
        .expect("cookie estática válida")
}

fn platform_cookie(headers: &HeaderMap) -> Result<&str, ApiError> {
    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|raw| {
            raw.split(';')
                .map(str::trim)
                .find_map(|part| part.strip_prefix("__Host-commercectrl_session="))
        })
        .filter(|token| !token.is_empty())
        .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "sessão administrativa ausente"))
}

fn tenant_session_cookie(token: &str) -> Result<HeaderValue, ApiError> {
    format!(
        "__Host-commercectrl_tenant_session={token}; Path=/; HttpOnly; Secure; SameSite=None; Max-Age=604800"
    )
    .parse()
    .map_err(internal)
}

fn expired_tenant_session_cookie() -> HeaderValue {
    "__Host-commercectrl_tenant_session=; Path=/; HttpOnly; Secure; SameSite=None; Max-Age=0"
        .parse()
        .expect("cookie estática válida")
}

fn tenant_cookie(headers: &HeaderMap) -> Result<&str, ApiError> {
    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|raw| {
            raw.split(';')
                .map(str::trim)
                .find_map(|part| part.strip_prefix("__Host-commercectrl_tenant_session="))
        })
        .filter(|token| !token.is_empty())
        .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "sessão de usuário ausente"))
}

impl FromRequestParts<Arc<AppState>> for PlatformAuth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let token = platform_cookie(&parts.headers)?;
        let row = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT a.id,a.email FROM platform_sessions s JOIN platform_admins a ON a.id=s.admin_id WHERE s.token_hash=$1 AND s.expires_at>now() AND a.active=true",
        )
        .bind(hash_key(token))
        .fetch_optional(&state.db)
        .await
        .map_err(internal)?
        .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "sessão administrativa expirada"))?;
        let _ = sqlx::query("UPDATE platform_sessions SET last_seen_at=now() WHERE token_hash=$1")
            .bind(hash_key(token))
            .execute(&state.db)
            .await;
        Ok(Self {
            admin_id: row.0,
            email: row.1,
        })
    }
}

/// Assinatura em dia (trial/cortesia/paga e período vigente). Quando falsa, o
/// painel continua acessível para leitura, mas qualquer escrita retorna 402.
async fn subscription_writable(pool: &PgPool, tenant_id: Uuid) -> Result<bool, ApiError> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM tenant_subscriptions WHERE tenant_id=$1 AND status IN ('trial','active') AND (current_period_ends_at IS NULL OR current_period_ends_at>now()))",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .map_err(internal)
}

impl FromRequestParts<Arc<AppState>> for TenantAuth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let token = tenant_cookie(&parts.headers)?;
        let row = sqlx::query_as::<_, (Uuid, Uuid, String, String)>(
            "SELECT u.id,u.tenant_id,u.email,u.role FROM tenant_sessions s JOIN tenant_users u ON u.id=s.user_id WHERE s.token_hash=$1 AND s.expires_at>now() AND u.active=true",
        )
        .bind(hash_key(token))
        .fetch_optional(&state.db)
        .await
        .map_err(internal)?
        .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "sessão de usuário expirada"))?;
        // Assinatura vencida/revogada: login permitido, mas somente leitura.
        if !subscription_writable(&state.db, row.1).await?
            && !parts.uri.path().starts_with("/api/v1/auth/")
            && !matches!(
                parts.method,
                axum::http::Method::GET | axum::http::Method::HEAD | axum::http::Method::OPTIONS
            )
        {
            return Err(error(
                StatusCode::PAYMENT_REQUIRED,
                "assinatura vencida: acesso somente leitura",
            ));
        }
        let _ = sqlx::query("UPDATE tenant_sessions SET last_seen_at=now() WHERE token_hash=$1")
            .bind(hash_key(token))
            .execute(&state.db)
            .await;
        Ok(Self {
            user_id: row.0,
            tenant_id: row.1,
            email: row.2,
            role: row.3,
        })
    }
}

/// Registra uma tentativa de login no banco e retorna true se ainda está dentro do limite permitido.
async fn record_login_attempt(
    state: &Arc<AppState>,
    email: &str,
    ip: &str,
    succeeded: bool,
    tenant_id: Option<Uuid>,
) -> Result<(), ApiError> {
    sqlx::query("INSERT INTO login_attempts(tenant_id,email,ip,succeeded) VALUES($1,$2,$3,$4)")
        .bind(tenant_id)
        .bind(email)
        .bind(ip)
        .bind(succeeded)
        .execute(&state.db)
        .await
        .map_err(internal)?;
    Ok(())
}

/// Conta tentativas falhadas recentes para um email ou IP.
async fn count_recent_failed_attempts(
    state: &Arc<AppState>,
    email: &str,
    ip: &str,
) -> Result<i64, ApiError> {
    let row: Option<LoginAttemptRecord> = sqlx::query_as(
        "SELECT COUNT(*)::bigint AS attempts FROM login_attempts WHERE succeeded=false AND attempted_at > now() - make_interval(secs => $1) AND (email=$2 OR ip=$3)",
    )
    .bind(LOGIN_RATE_LIMIT_WINDOW_SECS)
    .bind(email)
    .bind(ip)
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?;
    Ok(row.map(|r| r.attempts).unwrap_or(0))
}

async fn platform_login(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<PlatformLoginRequest>,
) -> Result<(HeaderMap, Json<Value>), ApiError> {
    let email = request.email.trim().to_lowercase();
    let ip = client_ip(&headers, addr);
    if email.is_empty() || request.password.len() < 8 {
        let _ = record_login_attempt(&state, &email, &ip, false, None).await;
        return Err(error(StatusCode::UNAUTHORIZED, "credenciais inválidas"));
    }

    let failures = count_recent_failed_attempts(&state, &email, &ip).await?;
    if failures >= MAX_LOGIN_ATTEMPTS {
        let _ = record_login_attempt(&state, &email, &ip, false, None).await;
        return Err(error(
            StatusCode::TOO_MANY_REQUESTS,
            "muitas tentativas de login; tente novamente mais tarde",
        ));
    }

    let admin = sqlx::query_as::<_, PlatformAdminRecord>(
        "SELECT id,email,password_hash FROM platform_admins WHERE email=$1 AND active=true",
    )
    .bind(&email)
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?
    .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "credenciais inválidas"))?;
    let parsed = PasswordHash::new(&admin.password_hash).map_err(|_| {
        error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "credencial administrativa inválida",
        )
    })?;
    let ok = Argon2::default()
        .verify_password(request.password.as_bytes(), &parsed)
        .is_ok();
    if !ok {
        let _ = record_login_attempt(&state, &email, &ip, false, None).await;
        return Err(error(StatusCode::UNAUTHORIZED, "credenciais inválidas"));
    }
    let _ = record_login_attempt(&state, &email, &ip, true, None).await;

    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    sqlx::query("DELETE FROM platform_sessions WHERE expires_at<=now()")
        .execute(&state.db)
        .await
        .map_err(internal)?;
    sqlx::query("INSERT INTO platform_sessions(token_hash,admin_id,expires_at) VALUES($1,$2,now()+interval '7 days')")
        .bind(hash_key(&token))
        .bind(admin.id)
        .execute(&state.db)
        .await
        .map_err(internal)?;
    sqlx::query("UPDATE platform_admins SET last_login_at=now() WHERE id=$1")
        .bind(admin.id)
        .execute(&state.db)
        .await
        .map_err(internal)?;
    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, platform_session_cookie(&token)?);
    Ok((
        headers,
        Json(serde_json::json!({"email":admin.email,"role":"superadmin"})),
    ))
}

async fn platform_logout(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    headers: HeaderMap,
) -> Result<(HeaderMap, Json<Value>), ApiError> {
    let token = platform_cookie(&headers)?;
    sqlx::query("DELETE FROM platform_sessions WHERE token_hash=$1 AND admin_id=$2")
        .bind(hash_key(token))
        .bind(auth.admin_id)
        .execute(&state.db)
        .await
        .map_err(internal)?;
    let mut response_headers = HeaderMap::new();
    response_headers.insert(header::SET_COOKIE, expired_platform_session_cookie());
    Ok((response_headers, Json(serde_json::json!({"ok":true}))))
}

async fn platform_me(auth: PlatformAuth) -> Json<Value> {
    Json(serde_json::json!({"email":auth.email,"role":"superadmin"}))
}

async fn tenant_login(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<TenantLoginRequest>,
) -> Result<(HeaderMap, Json<Value>), ApiError> {
    let email = request.email.trim().to_lowercase();
    let ip = client_ip(&headers, addr);
    if email.is_empty() || request.password.len() < 6 {
        let _ = record_login_attempt(&state, &email, &ip, false, None).await;
        return Err(error(StatusCode::UNAUTHORIZED, "credenciais inválidas"));
    }
    let slug = request
        .slug
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_lowercase);
    let candidates = sqlx::query_as::<_, TenantUserRecord>(
        "SELECT u.id,u.tenant_id,u.unit_id,u.email,u.password_hash,u.role,u.display_name,u.must_change_password FROM tenant_users u JOIN tenants t ON t.id=u.tenant_id WHERE u.email=$1 AND u.active=true AND u.revoked_at IS NULL AND t.archived_at IS NULL AND ($2::text IS NULL OR t.slug=$2) ORDER BY u.created_at",
    )
    .bind(&email)
    .bind(&slug)
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;

    if candidates.is_empty() {
        let _ = record_login_attempt(&state, &email, &ip, false, None).await;
        return Err(error(StatusCode::UNAUTHORIZED, "credenciais inválidas"));
    }
    if candidates.len() > 1 {
        // O mesmo e-mail existe em lojas diferentes: o cliente precisa informar o slug.
        let tenants = sqlx::query_as::<_, (String, String)>(
            "SELECT t.slug,t.name FROM tenant_users u JOIN tenants t ON t.id=u.tenant_id WHERE u.email=$1 AND u.active=true AND u.revoked_at IS NULL AND t.archived_at IS NULL ORDER BY t.name",
        )
        .bind(&email)
        .fetch_all(&state.db)
        .await
        .map_err(internal)?;
        let options: Vec<Value> = tenants
            .into_iter()
            .map(|(slug, name)| serde_json::json!({"slug": slug, "name": name}))
            .collect();
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "e-mail cadastrado em mais de uma loja; informe o slug",
                "tenants": options
            })),
        ));
    }
    let user = candidates.into_iter().next().expect("candidato único");
    let tenant_id = Some(user.tenant_id);

    let failures = count_recent_failed_attempts(&state, &email, &ip).await?;
    if failures >= MAX_LOGIN_ATTEMPTS {
        let _ = record_login_attempt(&state, &email, &ip, false, tenant_id).await;
        return Err(error(
            StatusCode::TOO_MANY_REQUESTS,
            "muitas tentativas de login; tente novamente mais tarde",
        ));
    }

    let parsed = PasswordHash::new(&user.password_hash).map_err(|_| {
        error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "credencial de usuário inválida",
        )
    })?;
    let ok = Argon2::default()
        .verify_password(request.password.as_bytes(), &parsed)
        .is_ok();
    if !ok {
        let _ = record_login_attempt(&state, &email, &ip, false, tenant_id).await;
        return Err(error(StatusCode::UNAUTHORIZED, "credenciais inválidas"));
    }
    let _ = record_login_attempt(&state, &email, &ip, true, tenant_id).await;

    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    sqlx::query("DELETE FROM tenant_sessions WHERE expires_at<=now()")
        .execute(&state.db)
        .await
        .map_err(internal)?;
    sqlx::query("INSERT INTO tenant_sessions(token_hash,user_id,expires_at) VALUES($1,$2,now()+interval '7 days')")
        .bind(hash_key(&token))
        .bind(user.id)
        .execute(&state.db)
        .await
        .map_err(internal)?;
    sqlx::query("UPDATE tenant_users SET last_login_at=now() WHERE id=$1")
        .bind(user.id)
        .execute(&state.db)
        .await
        .map_err(internal)?;
    let tenant_slug = sqlx::query_scalar::<_, String>("SELECT slug FROM tenants WHERE id=$1")
        .bind(user.tenant_id)
        .fetch_one(&state.db)
        .await
        .map_err(internal)?;
    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, tenant_session_cookie(&token)?);
    Ok((
        headers,
        Json(serde_json::json!({
            "email": user.email,
            "role": user.role,
            "tenant_id": user.tenant_id,
            "tenant_slug": tenant_slug,
            "unit_id": user.unit_id,
            "display_name": user.display_name,
            "must_change_password": user.must_change_password,
        })),
    ))
}

async fn tenant_change_password(
    State(state): State<Arc<AppState>>,
    auth: TenantAuth,
    Json(request): Json<TenantPasswordRequest>,
) -> Result<(HeaderMap, Json<Value>), ApiError> {
    if request.new_password.len() < 8 || request.new_password.len() > 128 {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "a nova senha deve ter entre 8 e 128 caracteres",
        ));
    }
    let stored = sqlx::query_as::<_, (String, bool)>(
        "SELECT password_hash,must_change_password FROM tenant_users WHERE id=$1 AND active=true",
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?
    .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "sessão de usuário expirada"))?;
    if !stored.1 {
        // Troca voluntária exige a senha atual; no primeiro acesso a própria
        // sessão autenticada basta (inclusive após recarregar a página).
        let parsed = PasswordHash::new(&stored.0).map_err(|_| {
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "credencial de usuário inválida",
            )
        })?;
        let ok = Argon2::default()
            .verify_password(request.current_password.as_bytes(), &parsed)
            .is_ok();
        if !ok {
            return Err(error(StatusCode::UNAUTHORIZED, "senha atual incorreta"));
        }
    }
    let new_hash = password_hash(&request.new_password)?;
    sqlx::query("UPDATE tenant_users SET password_hash=$1,must_change_password=false,updated_at=now() WHERE id=$2")
        .bind(new_hash)
        .bind(auth.user_id)
        .execute(&state.db)
        .await
        .map_err(internal)?;
    // Encerra todas as sessões antigas e emite um novo cookie, mantendo o
    // usuário logado após a troca (obrigatória ou voluntária).
    sqlx::query("DELETE FROM tenant_sessions WHERE user_id=$1")
        .bind(auth.user_id)
        .execute(&state.db)
        .await
        .map_err(internal)?;
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    sqlx::query("INSERT INTO tenant_sessions(token_hash,user_id,expires_at) VALUES($1,$2,now()+interval '7 days')")
        .bind(hash_key(&token))
        .bind(auth.user_id)
        .execute(&state.db)
        .await
        .map_err(internal)?;
    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, tenant_session_cookie(&token)?);
    Ok((headers, Json(serde_json::json!({"ok": true}))))
}

async fn tenant_logout(
    State(state): State<Arc<AppState>>,
    auth: TenantAuth,
    headers: HeaderMap,
) -> Result<(HeaderMap, Json<Value>), ApiError> {
    let token = tenant_cookie(&headers)?;
    sqlx::query("DELETE FROM tenant_sessions WHERE token_hash=$1 AND user_id=$2")
        .bind(hash_key(token))
        .bind(auth.user_id)
        .execute(&state.db)
        .await
        .map_err(internal)?;
    let mut response_headers = HeaderMap::new();
    response_headers.insert(header::SET_COOKIE, expired_tenant_session_cookie());
    Ok((response_headers, Json(serde_json::json!({"ok":true}))))
}

async fn tenant_me(
    State(state): State<Arc<AppState>>,
    auth: TenantAuth,
) -> Result<Json<Value>, ApiError> {
    let row = sqlx::query_as::<_, (Option<String>, bool, Option<String>)>(
        "SELECT u.display_name,u.must_change_password,t.slug FROM tenant_users u JOIN tenants t ON t.id=u.tenant_id WHERE u.id=$1",
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?
    .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "sessão de usuário expirada"))?;
    Ok(Json(serde_json::json!({
        "email": auth.email,
        "role": auth.role,
        "tenant_id": auth.tenant_id,
        "tenant_slug": row.2,
        "display_name": row.0,
        "must_change_password": row.1,
    })))
}

#[derive(Debug, Deserialize)]
struct TenantsQuery {
    #[serde(default)]
    include_archived: bool,
}

const TENANT_LIST_SQL: &str = r#"
SELECT t.id,t.name,t.slug,
    COUNT(DISTINCT u.id)::bigint AS units,
    COUNT(DISTINCT terminal.id)::bigint AS terminals,
    COUNT(DISTINCT terminal.id) FILTER (WHERE terminal.active AND terminal.last_seen_at>now()-interval '15 minutes')::bigint AS online_terminals,
    s.status AS subscription_status, s.billing_mode, s.plan,
    s.courtesy_until, s.current_period_ends_at, s.asaas_customer_id,
    (SELECT owner.email FROM tenant_users owner WHERE owner.tenant_id=t.id AND owner.role='owner' AND owner.active=true ORDER BY owner.created_at LIMIT 1) AS owner_email,
    t.cnpj_cpf, t.phone,
    COALESCE((SELECT p.monthly_brl_cents FROM platform_plan_catalog p WHERE p.code=CASE s.plan WHEN 'essencial' THEN 'starter' WHEN 'profissional' THEN 'fiscal' WHEN 'rede' THEN 'premium' ELSE s.plan END),0)::bigint AS monthly_brl_cents,
    (CASE WHEN s.billing_mode='paid' AND s.status='active' THEN COALESCE((SELECT p.monthly_brl_cents FROM platform_plan_catalog p WHERE p.code=CASE s.plan WHEN 'essencial' THEN 'starter' WHEN 'profissional' THEN 'fiscal' WHEN 'rede' THEN 'premium' ELSE s.plan END),0) ELSE 0 END)::bigint AS mrr_brl_cents,
    t.archived_at, t.created_at
FROM tenants t
LEFT JOIN units u ON u.tenant_id=t.id
LEFT JOIN terminals terminal ON terminal.tenant_id=t.id
LEFT JOIN tenant_subscriptions s ON s.tenant_id=t.id
WHERE ($1 OR t.archived_at IS NULL)
GROUP BY t.id,t.name,t.slug,s.status,s.billing_mode,s.plan,s.courtesy_until,s.current_period_ends_at,s.asaas_customer_id,t.cnpj_cpf,t.phone,t.archived_at,t.created_at
ORDER BY t.created_at DESC
"#;

async fn platform_tenants(
    State(state): State<Arc<AppState>>,
    _auth: PlatformAuth,
    Query(query): Query<TenantsQuery>,
) -> Result<Json<Vec<PlatformTenantOut>>, ApiError> {
    let tenants = sqlx::query_as::<_, PlatformTenantOut>(TENANT_LIST_SQL)
        .bind(query.include_archived)
        .fetch_all(&state.db)
        .await
        .map_err(internal)?;
    Ok(Json(tenants))
}

async fn platform_tenant_detail(
    State(state): State<Arc<AppState>>,
    _auth: PlatformAuth,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let unit_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM units WHERE tenant_id=$1 ORDER BY created_at ASC LIMIT 1",
    )
    .bind(tenant_id)
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?;
    let detail = sqlx::query_as::<
        _,
        (
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<chrono::DateTime<chrono::Utc>>,
            chrono::DateTime<chrono::Utc>,
            Option<String>,
        ),
    >(
        "SELECT t.name,t.slug,s.status,s.billing_mode,s.plan,s.courtesy_until,s.current_period_ends_at,s.asaas_customer_id,t.cnpj_cpf,t.phone,t.archived_at,t.created_at,(SELECT u.email FROM tenant_users u WHERE u.tenant_id=t.id AND u.role='owner' ORDER BY u.created_at LIMIT 1) FROM tenants t LEFT JOIN tenant_subscriptions s ON s.tenant_id=t.id WHERE t.id=$1",
    )
    .bind(tenant_id)
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?
    .ok_or_else(|| error(StatusCode::NOT_FOUND, "tenant não encontrado"))?;
    Ok(Json(serde_json::json!({
        "tenant_id": tenant_id,
        "unit_id": unit_id,
        "name": detail.0,
        "slug": detail.1,
        "subscription_status": detail.2,
        "billing_mode": detail.3,
        "plan": detail.4,
        "courtesy_until": detail.5,
        "current_period_ends_at": detail.6,
        "asaas_customer_id": detail.7,
        "cnpj_cpf": detail.8,
        "phone": detail.9,
        "archived_at": detail.10,
        "created_at": detail.11,
        "owner_email": detail.12,
    })))
}

// ---------------------------------------------------------------------------
// Operação do SuperAdmin: exclusão (arquivamento), plano, unidades, terminais,
// usuários, cobranças, auditoria, métricas, leads e feature flags.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ArchiveTenantRequest {
    confirm_slug: String,
    #[serde(default)]
    reason: Option<String>,
}

/// Arquiva um tenant (soft delete). Nunca apaga vendas: o histórico continua
/// disponível para auditoria e a loja pode ser restaurada.
async fn platform_archive_tenant(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<ArchiveTenantRequest>,
) -> Result<Json<Value>, ApiError> {
    let row = sqlx::query_as::<_, (Option<String>, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT slug,archived_at FROM tenants WHERE id=$1",
    )
    .bind(tenant_id)
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?
    .ok_or_else(|| error(StatusCode::NOT_FOUND, "tenant não encontrado"))?;
    if row.1.is_some() {
        return Err(error(StatusCode::CONFLICT, "tenant já está arquivado"));
    }
    let slug = row.0.unwrap_or_default();
    if request.confirm_slug.trim().to_lowercase() != slug.to_lowercase() {
        return Err(error(
            StatusCode::BAD_REQUEST,
            format!("confirmação inválida: digite o slug \"{slug}\" para arquivar"),
        ));
    }
    let reason = request
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(300).collect::<String>());

    let mut tx = state.db.begin().await.map_err(internal)?;
    let previous_status = sqlx::query_scalar::<_, Option<String>>(
        "SELECT status FROM tenant_subscriptions WHERE tenant_id=$1",
    )
    .bind(tenant_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?
    .flatten();
    sqlx::query("UPDATE tenants SET archived_at=now(),archived_by=$2,archive_reason=$3,archive_previous_status=$4 WHERE id=$1")
        .bind(tenant_id)
        .bind(auth.admin_id)
        .bind(&reason)
        .bind(&previous_status)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    sqlx::query("UPDATE tenant_subscriptions SET status='cancelled',updated_at=now() WHERE tenant_id=$1")
        .bind(tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    sqlx::query("UPDATE licenses SET status='suspended',updated_at=now() WHERE tenant_id=$1")
        .bind(tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    sqlx::query("UPDATE terminals SET active=false WHERE tenant_id=$1")
        .bind(tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    sqlx::query("UPDATE tenant_admins SET active=false WHERE tenant_id=$1")
        .bind(tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    sqlx::query("UPDATE platform_activation_codes SET revoked_at=now() WHERE tenant_id=$1 AND revoked_at IS NULL")
        .bind(tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    sqlx::query("DELETE FROM tenant_sessions WHERE user_id IN (SELECT id FROM tenant_users WHERE tenant_id=$1)")
        .bind(tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    platform_audit(
        &state.db,
        auth.admin_id,
        "tenant.archived",
        Some(tenant_id),
        serde_json::json!({"reason": reason, "previous_status": previous_status}),
    )
    .await;
    info!(admin=%auth.email,tenant=%tenant_id,"tenant arquivado (soft delete)");
    Ok(Json(serde_json::json!({
        "tenant_id": tenant_id,
        "archived": true,
        "archived_at": chrono::Utc::now()
    })))
}

/// Desfaz o arquivamento. Terminais continuam desligados e precisam de novo
/// código de ativação — mesma regra usada ao restaurar uma suspensão.
async fn platform_unarchive_tenant(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let row = sqlx::query_as::<
        _,
        (
            Option<chrono::DateTime<chrono::Utc>>,
            Option<String>,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<String>,
        ),
    >(
        "SELECT t.archived_at,t.archive_previous_status,s.trial_ends_at,s.courtesy_until,s.billing_mode FROM tenants t LEFT JOIN tenant_subscriptions s ON s.tenant_id=t.id WHERE t.id=$1",
    )
    .bind(tenant_id)
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?
    .ok_or_else(|| error(StatusCode::NOT_FOUND, "tenant não encontrado"))?;
    if row.0.is_none() {
        return Err(error(StatusCode::CONFLICT, "tenant não está arquivado"));
    }
    let now = chrono::Utc::now();
    let restored_status = row
        .1
        .clone()
        .filter(|status| status != "cancelled")
        .unwrap_or_else(|| {
            if row.2.is_some_and(|trial| trial > now) {
                "trial".into()
            } else {
                "active".into()
            }
        });
    let mut tx = state.db.begin().await.map_err(internal)?;
    sqlx::query("UPDATE tenants SET archived_at=NULL,archived_by=NULL,archive_reason=NULL,archive_previous_status=NULL WHERE id=$1")
        .bind(tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    sqlx::query("UPDATE tenant_subscriptions SET status=$2,updated_at=now() WHERE tenant_id=$1")
        .bind(tenant_id)
        .bind(&restored_status)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    sqlx::query("UPDATE tenant_admins SET active=true WHERE tenant_id=$1")
        .bind(tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    // Assinatura de cortesia vencida não deve voltar como ativa.
    if row.4.as_deref() == Some("courtesy") && row.3.is_some_and(|until| until <= now) {
        sqlx::query("UPDATE tenant_subscriptions SET status='suspended',updated_at=now() WHERE tenant_id=$1")
            .bind(tenant_id)
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
    }
    tx.commit().await.map_err(internal)?;
    platform_audit(
        &state.db,
        auth.admin_id,
        "tenant.unarchived",
        Some(tenant_id),
        serde_json::json!({"status": restored_status}),
    )
    .await;
    info!(admin=%auth.email,tenant=%tenant_id,"tenant desarquivado");
    Ok(Json(serde_json::json!({
        "tenant_id": tenant_id,
        "archived": false,
        "status": restored_status,
        "terminals_require_reactivation": true
    })))
}

#[derive(Debug, Deserialize)]
struct TenantUpdateRequest {
    name: Option<String>,
    slug: Option<String>,
    cnpj_cpf: Option<String>,
    phone: Option<String>,
    owner_email: Option<String>,
}

async fn platform_update_tenant(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<TenantUpdateRequest>,
) -> Result<Json<Value>, ApiError> {
    let current = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT name,slug FROM tenants WHERE id=$1",
    )
    .bind(tenant_id)
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?
    .ok_or_else(|| error(StatusCode::NOT_FOUND, "tenant não encontrado"))?;
    let mut changed = Vec::new();
    let name = match request.name {
        Some(value) => {
            let value = value.trim().to_string();
            if value.len() < 2 || value.len() > 120 {
                return Err(error(StatusCode::BAD_REQUEST, "nome inválido"));
            }
            if value != current.0 {
                changed.push("name");
            }
            Some(value)
        }
        None => None,
    };
    let slug = match request.slug {
        Some(value) => {
            let value = slugify(value.trim());
            if value.is_empty() || value.len() > 80 {
                return Err(error(StatusCode::BAD_REQUEST, "slug inválido"));
            }
            let taken = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM tenants WHERE slug=$1 AND id<>$2)",
            )
            .bind(&value)
            .bind(tenant_id)
            .fetch_one(&state.db)
            .await
            .map_err(internal)?;
            if taken {
                return Err(error(StatusCode::CONFLICT, "slug já usado por outra loja"));
            }
            changed.push("slug");
            Some(value)
        }
        None => None,
    };
    let cnpj = request.cnpj_cpf.map(|value| {
        let digits = value.chars().filter(char::is_ascii_digit).collect::<String>();
        if digits.is_empty() {
            None
        } else {
            Some(value.trim().chars().take(24).collect::<String>())
        }
    });
    let phone = request.phone.map(|value| {
        let trimmed = value.trim().chars().take(32).collect::<String>();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });
    if let Some(Some(_)) = &cnpj {
        changed.push("cnpj_cpf");
    }
    if let Some(Some(_)) = &phone {
        changed.push("phone");
    }
    let owner_email = request.owner_email.map(|value| value.trim().to_lowercase());
    if let Some(email) = &owner_email {
        let valid = email.len() >= 5
            && email.len() <= 200
            && email
                .split_once('@')
                .is_some_and(|(user, domain)| !user.is_empty() && domain.contains('.'));
        if !valid {
            return Err(error(StatusCode::BAD_REQUEST, "e-mail do responsável inválido"));
        }
        changed.push("owner_email");
    }

    let mut tx = state.db.begin().await.map_err(internal)?;
    sqlx::query(
        "UPDATE tenants SET name=COALESCE($2,name),slug=COALESCE($3,slug),cnpj_cpf=CASE WHEN $4 THEN $5 ELSE cnpj_cpf END,phone=CASE WHEN $6 THEN $7 ELSE phone END WHERE id=$1",
    )
    .bind(tenant_id)
    .bind(&name)
    .bind(&slug)
    .bind(cnpj.is_some())
    .bind(cnpj.clone().flatten())
    .bind(phone.is_some())
    .bind(phone.clone().flatten())
    .execute(&mut *tx)
    .await
    .map_err(internal)?;
    if let Some(email) = &owner_email {
        sqlx::query("UPDATE tenant_users SET email=$2,updated_at=now() WHERE tenant_id=$1 AND role='owner'")
            .bind(tenant_id)
            .bind(email)
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
        sqlx::query("UPDATE tenant_admins SET email=$2 WHERE tenant_id=$1 AND role='owner'")
            .bind(tenant_id)
            .bind(email)
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
    }
    tx.commit().await.map_err(internal)?;
    platform_audit(
        &state.db,
        auth.admin_id,
        "tenant.updated",
        Some(tenant_id),
        serde_json::json!({"fields": changed}),
    )
    .await;
    Ok(Json(serde_json::json!({"tenant_id": tenant_id, "updated": changed})))
}

#[derive(Debug, Deserialize)]
struct PlanChangeRequest {
    plan: String,
    note: Option<String>,
}

async fn platform_change_plan(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<PlanChangeRequest>,
) -> Result<Json<Value>, ApiError> {
    let plan = request.plan.trim().to_lowercase();
    let valid = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM platform_plan_catalog WHERE code=$1 AND active=true)",
    )
    .bind(&plan)
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;
    if !valid {
        return Err(error(StatusCode::BAD_REQUEST, "plano inválido ou inativo"));
    }
    let current = sqlx::query_scalar::<_, String>(
        "SELECT plan FROM tenant_subscriptions WHERE tenant_id=$1",
    )
    .bind(tenant_id)
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?
    .ok_or_else(|| error(StatusCode::NOT_FOUND, "assinatura não encontrada"))?;
    let note = request
        .note
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(300).collect::<String>());
    let mut tx = state.db.begin().await.map_err(internal)?;
    sqlx::query("UPDATE tenant_subscriptions SET plan=$2,updated_at=now() WHERE tenant_id=$1")
        .bind(tenant_id)
        .bind(&plan)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    sqlx::query("INSERT INTO tenant_plan_history(tenant_id,from_plan,to_plan,changed_by,note) VALUES($1,$2,$3,$4,$5)")
        .bind(tenant_id)
        .bind(&current)
        .bind(&plan)
        .bind(auth.admin_id)
        .bind(&note)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    platform_audit(
        &state.db,
        auth.admin_id,
        "tenant.plan_changed",
        Some(tenant_id),
        serde_json::json!({"from": current, "to": plan, "note": note}),
    )
    .await;
    Ok(Json(serde_json::json!({
        "tenant_id": tenant_id,
        "plan": plan,
        "previous_plan": current
    })))
}

async fn platform_plan_history(
    State(state): State<Arc<AppState>>,
    _auth: PlatformAuth,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<_, (i64, Option<String>, String, Option<String>, Option<String>, chrono::DateTime<chrono::Utc>)>(
        "SELECT h.id,h.from_plan,h.to_plan,a.email,h.note,h.created_at FROM tenant_plan_history h LEFT JOIN platform_admins a ON a.id=h.changed_by WHERE h.tenant_id=$1 ORDER BY h.created_at DESC LIMIT 50",
    )
    .bind(tenant_id)
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;
    Ok(Json(serde_json::json!(rows
        .into_iter()
        .map(|row| serde_json::json!({
            "id": row.0,
            "from_plan": row.1,
            "to_plan": row.2,
            "changed_by": row.3,
            "note": row.4,
            "created_at": row.5,
        }))
        .collect::<Vec<_>>())))
}

#[derive(Debug, Deserialize)]
struct OwnerPasswordRequest {
    #[serde(default)]
    new_password: Option<String>,
}

/// Redefine a senha do owner. Sem `new_password`, gera uma temporária e exige
/// troca no próximo login.
async fn platform_reset_owner_password(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<OwnerPasswordRequest>,
) -> Result<Json<Value>, ApiError> {
    let owner = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM tenant_users WHERE tenant_id=$1 AND role='owner' ORDER BY created_at LIMIT 1",
    )
    .bind(tenant_id)
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?
    .ok_or_else(|| error(StatusCode::NOT_FOUND, "owner do tenant não encontrado"))?;
    let provided = request
        .new_password
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(password) = provided {
        if password.len() < 8 || password.len() > 128 {
            return Err(error(
                StatusCode::BAD_REQUEST,
                "a senha deve ter entre 8 e 128 caracteres",
            ));
        }
    }
    let password = provided
        .map(str::to_string)
        .unwrap_or_else(temporary_password);
    let hash = password_hash(&password)?;
    let mut tx = state.db.begin().await.map_err(internal)?;
    sqlx::query("UPDATE tenant_users SET password_hash=$2,must_change_password=$3,active=true,revoked_at=NULL,updated_at=now() WHERE id=$1")
        .bind(owner)
        .bind(&hash)
        .bind(provided.is_none())
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    sqlx::query("DELETE FROM tenant_sessions WHERE user_id=$1")
        .bind(owner)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    platform_audit(
        &state.db,
        auth.admin_id,
        "tenant.owner_password_reset",
        Some(tenant_id),
        serde_json::json!({"generated": provided.is_none()}),
    )
    .await;
    Ok(Json(serde_json::json!({
        "tenant_id": tenant_id,
        "password": password,
        "generated": provided.is_none(),
        "must_change_password": provided.is_none()
    })))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct PlatformUnitOut {
    id: Uuid,
    name: String,
    code: String,
    active: bool,
    terminals: i64,
}

async fn platform_tenant_units(
    State(state): State<Arc<AppState>>,
    _auth: PlatformAuth,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<Vec<PlatformUnitOut>>, ApiError> {
    let rows = sqlx::query_as::<_, PlatformUnitOut>(
        "SELECT u.id,u.name,u.code,u.active,COUNT(t.id)::bigint AS terminals FROM units u LEFT JOIN terminals t ON t.unit_id=u.id AND t.tenant_id=u.tenant_id WHERE u.tenant_id=$1 GROUP BY u.id,u.name,u.code,u.active ORDER BY u.created_at",
    )
    .bind(tenant_id)
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
struct UnitCreateRequest {
    name: String,
    #[serde(default)]
    code: Option<String>,
}

async fn platform_create_unit(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<UnitCreateRequest>,
) -> Result<Json<Value>, ApiError> {
    let name = request.name.trim();
    if name.len() < 2 || name.len() > 120 {
        return Err(error(StatusCode::BAD_REQUEST, "nome da unidade inválido"));
    }
    let code = request
        .code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| slugify(value).to_uppercase())
        .unwrap_or_else(|| format!("UNIDADE{}", Uuid::new_v4().simple().to_string()[..4].to_uppercase()));
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM tenants WHERE id=$1 AND archived_at IS NULL)",
    )
    .bind(tenant_id)
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;
    if !exists {
        return Err(error(StatusCode::NOT_FOUND, "tenant não encontrado ou arquivado"));
    }
    let id = Uuid::new_v4();
    let inserted = sqlx::query(
        "INSERT INTO units(id,tenant_id,name,code) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING",
    )
    .bind(id)
    .bind(tenant_id)
    .bind(name)
    .bind(&code)
    .execute(&state.db)
    .await
    .map_err(internal)?
    .rows_affected();
    if inserted == 0 {
        return Err(error(
            StatusCode::CONFLICT,
            "já existe uma unidade com esse código nesta loja",
        ));
    }
    platform_audit(
        &state.db,
        auth.admin_id,
        "unit.created",
        Some(tenant_id),
        serde_json::json!({"unit_id": id, "name": name, "code": code}),
    )
    .await;
    Ok(Json(serde_json::json!({
        "id": id,
        "name": name,
        "code": code,
        "active": true,
        "terminals": 0
    })))
}

#[derive(Debug, Deserialize)]
struct UnitUpdateRequest {
    name: Option<String>,
    active: Option<bool>,
}

async fn platform_update_unit(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    Path((tenant_id, unit_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UnitUpdateRequest>,
) -> Result<Json<Value>, ApiError> {
    if let Some(name) = &request.name {
        let name = name.trim();
        if name.len() < 2 || name.len() > 120 {
            return Err(error(StatusCode::BAD_REQUEST, "nome da unidade inválido"));
        }
    }
    let changed = sqlx::query(
        "UPDATE units SET name=COALESCE($3,name),active=COALESCE($4,active) WHERE tenant_id=$1 AND id=$2",
    )
    .bind(tenant_id)
    .bind(unit_id)
    .bind(request.name.as_deref().map(str::trim))
    .bind(request.active)
    .execute(&state.db)
    .await
    .map_err(internal)?
    .rows_affected();
    if changed == 0 {
        return Err(error(StatusCode::NOT_FOUND, "unidade não encontrada"));
    }
    if request.active == Some(false) {
        sqlx::query("UPDATE terminals SET active=false WHERE tenant_id=$1 AND unit_id=$2")
            .bind(tenant_id)
            .bind(unit_id)
            .execute(&state.db)
            .await
            .map_err(internal)?;
    }
    platform_audit(
        &state.db,
        auth.admin_id,
        "unit.updated",
        Some(tenant_id),
        serde_json::json!({"unit_id": unit_id, "name": request.name, "active": request.active}),
    )
    .await;
    Ok(Json(serde_json::json!({"unit_id": unit_id, "updated": true})))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct PlatformTerminalOut {
    id: Uuid,
    unit_id: Uuid,
    unit_name: String,
    name: String,
    active: bool,
    last_seen_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
}

async fn platform_tenant_terminals(
    State(state): State<Arc<AppState>>,
    _auth: PlatformAuth,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<Vec<PlatformTerminalOut>>, ApiError> {
    let rows = sqlx::query_as::<_, PlatformTerminalOut>(
        "SELECT t.id,t.unit_id,u.name AS unit_name,t.name,t.active,t.last_seen_at,t.created_at FROM terminals t JOIN units u ON u.id=t.unit_id AND u.tenant_id=t.tenant_id WHERE t.tenant_id=$1 ORDER BY t.created_at DESC",
    )
    .bind(tenant_id)
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
struct TerminalUpdateRequest {
    name: Option<String>,
    active: Option<bool>,
}

async fn platform_update_terminal(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    Path((tenant_id, terminal_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<TerminalUpdateRequest>,
) -> Result<Json<Value>, ApiError> {
    if let Some(name) = &request.name {
        let name = name.trim();
        if name.len() < 2 || name.len() > 120 {
            return Err(error(StatusCode::BAD_REQUEST, "nome do terminal inválido"));
        }
    }
    let changed = sqlx::query(
        "UPDATE terminals SET name=COALESCE($3,name),active=COALESCE($4,active) WHERE tenant_id=$1 AND id=$2",
    )
    .bind(tenant_id)
    .bind(terminal_id)
    .bind(request.name.as_deref().map(str::trim))
    .bind(request.active)
    .execute(&state.db)
    .await
    .map_err(internal)?
    .rows_affected();
    if changed == 0 {
        return Err(error(StatusCode::NOT_FOUND, "terminal não encontrado"));
    }
    if request.active == Some(false) {
        sqlx::query("UPDATE licenses SET status='suspended',updated_at=now() WHERE tenant_id=$1 AND unit_id=(SELECT unit_id FROM terminals WHERE id=$2)")
            .bind(tenant_id)
            .bind(terminal_id)
            .execute(&state.db)
            .await
            .map_err(internal)?;
    }
    platform_audit(
        &state.db,
        auth.admin_id,
        "terminal.updated",
        Some(tenant_id),
        serde_json::json!({"terminal_id": terminal_id, "name": request.name, "active": request.active}),
    )
    .await;
    Ok(Json(serde_json::json!({"terminal_id": terminal_id, "updated": true})))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct PlatformUserOut {
    id: Uuid,
    email: String,
    role: String,
    active: bool,
    unit_id: Option<Uuid>,
    display_name: Option<String>,
    must_change_password: bool,
    last_login_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
}

async fn platform_tenant_users(
    State(state): State<Arc<AppState>>,
    _auth: PlatformAuth,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<Vec<PlatformUserOut>>, ApiError> {
    let rows = sqlx::query_as::<_, PlatformUserOut>(
        "SELECT id,email,role,active,unit_id,display_name,must_change_password,last_login_at,created_at FROM tenant_users WHERE tenant_id=$1 ORDER BY (role='owner') DESC,created_at",
    )
    .bind(tenant_id)
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
struct PlatformUserUpdateRequest {
    role: Option<String>,
    active: Option<bool>,
    display_name: Option<String>,
}

async fn platform_update_user(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    Path((tenant_id, user_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<PlatformUserUpdateRequest>,
) -> Result<Json<Value>, ApiError> {
    if let Some(role) = &request.role {
        if !matches!(
            role.as_str(),
            "owner" | "manager" | "cashier" | "stock" | "finance" | "viewer"
        ) {
            return Err(error(StatusCode::BAD_REQUEST, "papel inválido"));
        }
    }
    if request.active == Some(false) {
        let is_owner = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM tenant_users WHERE tenant_id=$1 AND id=$2 AND role='owner')",
        )
        .bind(tenant_id)
        .bind(user_id)
        .fetch_one(&state.db)
        .await
        .map_err(internal)?;
        if is_owner {
            return Err(error(
                StatusCode::CONFLICT,
                "o owner não pode ser desativado; redefina a senha ou arquive a loja",
            ));
        }
    }
    let changed = sqlx::query(
        "UPDATE tenant_users SET role=COALESCE($3,role),active=COALESCE($4,active),display_name=COALESCE($5,display_name),updated_at=now() WHERE tenant_id=$1 AND id=$2",
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(request.role.as_deref())
    .bind(request.active)
    .bind(request.display_name.as_deref().map(str::trim))
    .execute(&state.db)
    .await
    .map_err(internal)?
    .rows_affected();
    if changed == 0 {
        return Err(error(StatusCode::NOT_FOUND, "usuário não encontrado"));
    }
    if request.active == Some(false) {
        sqlx::query("DELETE FROM tenant_sessions WHERE user_id=$1")
            .bind(user_id)
            .execute(&state.db)
            .await
            .map_err(internal)?;
    }
    platform_audit(
        &state.db,
        auth.admin_id,
        "tenant_user.updated",
        Some(tenant_id),
        serde_json::json!({"user_id": user_id, "role": request.role, "active": request.active}),
    )
    .await;
    Ok(Json(serde_json::json!({"user_id": user_id, "updated": true})))
}

async fn platform_reset_user_password(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    Path((tenant_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>, ApiError> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM tenant_users WHERE tenant_id=$1 AND id=$2)",
    )
    .bind(tenant_id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;
    if !exists {
        return Err(error(StatusCode::NOT_FOUND, "usuário não encontrado"));
    }
    let password = temporary_password();
    let hash = password_hash(&password)?;
    let mut tx = state.db.begin().await.map_err(internal)?;
    sqlx::query("UPDATE tenant_users SET password_hash=$3,must_change_password=true,active=true,revoked_at=NULL,updated_at=now() WHERE tenant_id=$1 AND id=$2")
        .bind(tenant_id)
        .bind(user_id)
        .bind(hash)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    sqlx::query("DELETE FROM tenant_sessions WHERE user_id=$1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    platform_audit(
        &state.db,
        auth.admin_id,
        "tenant_user.password_reset",
        Some(tenant_id),
        serde_json::json!({"user_id": user_id}),
    )
    .await;
    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "password": password,
        "must_change_password": true
    })))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct PlatformPaymentOut {
    id: Uuid,
    amount_brl_cents: i64,
    status: String,
    provider_id: Option<String>,
    external_reference: String,
    asaas_status: Option<String>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
}

async fn platform_tenant_payments(
    State(state): State<Arc<AppState>>,
    _auth: PlatformAuth,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<Vec<PlatformPaymentOut>>, ApiError> {
    let rows = sqlx::query_as::<_, PlatformPaymentOut>(
        "SELECT id,amount_brl_cents,status,provider_id,external_reference,(raw->>'asaas_status') AS asaas_status,expires_at,created_at FROM payment_intents WHERE tenant_id=$1 AND provider='asaas' ORDER BY created_at DESC LIMIT 100",
    )
    .bind(tenant_id)
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;
    Ok(Json(rows))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct BillingQueueRow {
    tenant_id: Uuid,
    name: String,
    slug: Option<String>,
    plan: Option<String>,
    status: Option<String>,
    billing_mode: Option<String>,
    current_period_ends_at: Option<chrono::DateTime<chrono::Utc>>,
    courtesy_until: Option<chrono::DateTime<chrono::Utc>>,
    asaas_customer_id: Option<String>,
    monthly_brl_cents: i64,
    days_left: Option<i32>,
    pending_amount_cents: Option<i64>,
    pending_payment_id: Option<Uuid>,
}

async fn platform_billing_queue(
    State(state): State<Arc<AppState>>,
    _auth: PlatformAuth,
) -> Result<Json<Vec<BillingQueueRow>>, ApiError> {
    let rows = sqlx::query_as::<_, BillingQueueRow>(
        r#"
        SELECT t.id AS tenant_id,t.name,t.slug,s.plan,s.status,s.billing_mode,
            s.current_period_ends_at,s.courtesy_until,s.asaas_customer_id,
            COALESCE((SELECT p.monthly_brl_cents FROM platform_plan_catalog p WHERE p.code=CASE s.plan WHEN 'essencial' THEN 'starter' WHEN 'profissional' THEN 'fiscal' WHEN 'rede' THEN 'premium' ELSE s.plan END),0)::bigint AS monthly_brl_cents,
            (CASE WHEN COALESCE(s.courtesy_until,s.current_period_ends_at) IS NULL THEN NULL ELSE (COALESCE(s.courtesy_until,s.current_period_ends_at)::date - now()::date) END)::int AS days_left,
            (SELECT pi.amount_brl_cents FROM payment_intents pi WHERE pi.tenant_id=t.id AND pi.provider='asaas' AND pi.status IN ('creating','pending') ORDER BY pi.created_at DESC LIMIT 1) AS pending_amount_cents,
            (SELECT pi.id FROM payment_intents pi WHERE pi.tenant_id=t.id AND pi.provider='asaas' AND pi.status IN ('creating','pending') ORDER BY pi.created_at DESC LIMIT 1) AS pending_payment_id
        FROM tenants t JOIN tenant_subscriptions s ON s.tenant_id=t.id
        WHERE t.archived_at IS NULL AND (
            s.status IN ('past_due','suspended')
            OR s.status='trial'
            OR (s.current_period_ends_at IS NOT NULL AND s.current_period_ends_at <= now()+interval '10 days')
            OR (s.billing_mode='courtesy' AND s.courtesy_until IS NOT NULL AND s.courtesy_until <= now()+interval '30 days')
        )
        ORDER BY COALESCE(s.courtesy_until,s.current_period_ends_at) NULLS LAST
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
struct AuditQuery {
    tenant_id: Option<Uuid>,
    action: Option<String>,
    #[serde(default = "default_audit_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn default_audit_limit() -> i64 {
    100
}

async fn platform_audit_list(
    State(state): State<Arc<AppState>>,
    _auth: PlatformAuth,
    Query(query): Query<AuditQuery>,
) -> Result<Json<Value>, ApiError> {
    let limit = query.limit.clamp(1, 500);
    let offset = query.offset.max(0);
    let action = query
        .action
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("{value}%"));
    let rows = sqlx::query_as::<_, (i64, String, Option<String>, Option<Uuid>, Option<String>, String, chrono::DateTime<chrono::Utc>)>(
        r#"SELECT e.id,e.action,a.email,e.tenant_id,t.name,e.metadata::text,e.created_at
           FROM platform_audit_events e
           LEFT JOIN platform_admins a ON a.id=e.admin_id
           LEFT JOIN tenants t ON t.id=e.tenant_id
           WHERE ($1::uuid IS NULL OR e.tenant_id=$1)
             AND ($2::text IS NULL OR e.action LIKE $2)
           ORDER BY e.created_at DESC LIMIT $3 OFFSET $4"#,
    )
    .bind(query.tenant_id)
    .bind(&action)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;
    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)::bigint FROM platform_audit_events e WHERE ($1::uuid IS NULL OR e.tenant_id=$1) AND ($2::text IS NULL OR e.action LIKE $2)",
    )
    .bind(query.tenant_id)
    .bind(&action)
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;
    Ok(Json(serde_json::json!({
        "total": total,
        "items": rows.into_iter().map(|row| serde_json::json!({
            "id": row.0,
            "action": row.1,
            "admin_email": row.2,
            "tenant_id": row.3,
            "tenant_name": row.4,
            "metadata": serde_json::from_str::<Value>(&row.5).unwrap_or(Value::Null),
            "created_at": row.6,
        })).collect::<Vec<_>>()
    })))
}

async fn platform_metrics(
    State(state): State<Arc<AppState>>,
    _auth: PlatformAuth,
) -> Result<Json<Value>, ApiError> {
    let counts = sqlx::query_as::<
        _,
        (
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
        ),
    >(
        r#"SELECT
            COUNT(*) FILTER (WHERE t.archived_at IS NULL)::bigint,
            COUNT(*) FILTER (WHERE t.archived_at IS NULL AND s.status IN ('trial','active'))::bigint,
            COUNT(*) FILTER (WHERE t.archived_at IS NULL AND s.status='trial')::bigint,
            COUNT(*) FILTER (WHERE t.archived_at IS NULL AND s.status='past_due')::bigint,
            COUNT(*) FILTER (WHERE t.archived_at IS NULL AND s.status='suspended')::bigint,
            COUNT(*) FILTER (WHERE t.archived_at IS NULL AND s.billing_mode='courtesy')::bigint,
            COUNT(*) FILTER (WHERE t.archived_at IS NOT NULL)::bigint,
            COUNT(*) FILTER (WHERE t.created_at > now()-interval '30 days')::bigint,
            COUNT(*) FILTER (WHERE t.archived_at > now()-interval '30 days')::bigint
        FROM tenants t LEFT JOIN tenant_subscriptions s ON s.tenant_id=t.id"#,
    )
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;

    let plan_price = "COALESCE((SELECT p.monthly_brl_cents FROM platform_plan_catalog p WHERE p.code=CASE s.plan WHEN 'essencial' THEN 'starter' WHEN 'profissional' THEN 'fiscal' WHEN 'rede' THEN 'premium' ELSE s.plan END),0)";
    let mrr = sqlx::query_as::<_, (i64, i64)>(&format!(
        "SELECT COALESCE(SUM(CASE WHEN s.billing_mode='paid' AND s.status='active' THEN {plan_price} ELSE 0 END),0)::bigint, COALESCE(SUM(CASE WHEN s.status IN ('past_due','suspended') THEN {plan_price} ELSE 0 END),0)::bigint FROM tenant_subscriptions s JOIN tenants t ON t.id=s.tenant_id WHERE t.archived_at IS NULL"
    ))
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;

    let terminals = sqlx::query_as::<_, (i64, i64)>(
        "SELECT COUNT(*)::bigint, COUNT(*) FILTER (WHERE t.active AND t.last_seen_at>now()-interval '15 minutes')::bigint FROM terminals t JOIN tenants tn ON tn.id=t.tenant_id WHERE tn.archived_at IS NULL",
    )
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;

    let gmv = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(total_brl_cents),0)::bigint FROM sales WHERE received_at>now()-interval '30 days'",
    )
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;

    let received = sqlx::query_as::<_, (i64, i64)>(
        "SELECT COUNT(*)::bigint, COALESCE(SUM(amount_brl_cents),0)::bigint FROM payment_intents WHERE provider='asaas' AND status IN ('received','confirmed') AND updated_at>now()-interval '30 days'",
    )
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;

    Ok(Json(serde_json::json!({
        "tenants": counts.0,
        "active_subscriptions": counts.1,
        "trials": counts.2,
        "past_due": counts.3,
        "suspended": counts.4,
        "courtesy": counts.5,
        "archived": counts.6,
        "new_30d": counts.7,
        "archived_30d": counts.8,
        "mrr_cents": mrr.0,
        "arr_cents": mrr.0 * 12,
        "mrr_at_risk_cents": mrr.1,
        "terminals": terminals.0,
        "online_terminals": terminals.1,
        "gmv_30d_cents": gmv,
        "payments_received_30d": received.0,
        "revenue_received_30d_cents": received.1,
    })))
}

#[derive(Debug, Deserialize)]
struct AnalyticsQuery {
    #[serde(default = "default_analytics_months")]
    months: i64,
}

fn default_analytics_months() -> i64 {
    6
}

async fn platform_analytics(
    State(state): State<Arc<AppState>>,
    _auth: PlatformAuth,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<Value>, ApiError> {
    let months = query.months.clamp(3, 24);
    let series = |subject: &'static str| {
        let sql = match subject {
            "tenants" => {
                "SELECT to_char(m,'YYYY-MM') AS label,(SELECT COUNT(*) FROM tenants t WHERE date_trunc('month',t.created_at)=m AND t.archived_at IS NULL)::bigint AS value FROM generate_series(date_trunc('month',now())-(($1::int)-1)*interval '1 month',date_trunc('month',now()),interval '1 month') AS m"
            }
            "revenue" => {
                "SELECT to_char(m,'YYYY-MM') AS label,COALESCE((SELECT SUM(pi.amount_brl_cents) FROM payment_intents pi WHERE date_trunc('month',pi.updated_at)=m AND pi.provider='asaas' AND pi.status IN ('received','confirmed')),0)::bigint AS value FROM generate_series(date_trunc('month',now())-(($1::int)-1)*interval '1 month',date_trunc('month',now()),interval '1 month') AS m"
            }
            _ => {
                "SELECT to_char(m,'YYYY-MM') AS label,COALESCE((SELECT SUM(s.total_brl_cents) FROM sales s WHERE date_trunc('month',s.received_at)=m),0)::bigint AS value FROM generate_series(date_trunc('month',now())-(($1::int)-1)*interval '1 month',date_trunc('month',now()),interval '1 month') AS m"
            }
        };
        sql
    };
    let new_tenants = sqlx::query_as::<_, (String, i64)>(series("tenants"))
        .bind(months)
        .fetch_all(&state.db)
        .await
        .map_err(internal)?;
    let revenue = sqlx::query_as::<_, (String, i64)>(series("revenue"))
        .bind(months)
        .fetch_all(&state.db)
        .await
        .map_err(internal)?;
    let gmv = sqlx::query_as::<_, (String, i64)>(series("gmv"))
        .bind(months)
        .fetch_all(&state.db)
        .await
        .map_err(internal)?;
    let plans = sqlx::query_as::<_, (String, i64, i64)>(
        r#"SELECT s.plan,COUNT(*)::bigint,
           COALESCE(SUM(CASE WHEN s.status='active' AND s.billing_mode='paid' THEN COALESCE((SELECT p.monthly_brl_cents FROM platform_plan_catalog p WHERE p.code=CASE s.plan WHEN 'essencial' THEN 'starter' WHEN 'profissional' THEN 'fiscal' WHEN 'rede' THEN 'premium' ELSE s.plan END),0) ELSE 0 END),0)::bigint
           FROM tenant_subscriptions s JOIN tenants t ON t.id=s.tenant_id
           WHERE t.archived_at IS NULL GROUP BY s.plan ORDER BY COUNT(*) DESC"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;
    let statuses = sqlx::query_as::<_, (String, i64)>(
        "SELECT COALESCE(s.status,'sem_assinatura'),COUNT(*)::bigint FROM tenants t LEFT JOIN tenant_subscriptions s ON s.tenant_id=t.id WHERE t.archived_at IS NULL GROUP BY 1 ORDER BY 2 DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;
    Ok(Json(serde_json::json!({
        "new_tenants": new_tenants.into_iter().map(|row| serde_json::json!({"label": row.0, "value": row.1})).collect::<Vec<_>>(),
        "revenue": revenue.into_iter().map(|row| serde_json::json!({"label": row.0, "value": row.1})).collect::<Vec<_>>(),
        "gmv": gmv.into_iter().map(|row| serde_json::json!({"label": row.0, "value": row.1})).collect::<Vec<_>>(),
        "plans": plans.into_iter().map(|row| serde_json::json!({"plan": row.0, "count": row.1, "mrr_cents": row.2})).collect::<Vec<_>>(),
        "statuses": statuses.into_iter().map(|row| serde_json::json!({"status": row.0, "count": row.1})).collect::<Vec<_>>(),
    })))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct PlatformLeadOut {
    id: Uuid,
    store_name: String,
    contact_name: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    city: Option<String>,
    plan_code: Option<String>,
    modality: String,
    details: Option<String>,
    status: String,
    notes: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

async fn platform_leads(
    State(state): State<Arc<AppState>>,
    _auth: PlatformAuth,
) -> Result<Json<Vec<PlatformLeadOut>>, ApiError> {
    let rows = sqlx::query_as::<_, PlatformLeadOut>(
        "SELECT id,store_name,contact_name,email,phone,city,plan_code,modality,details,status,notes,created_at FROM commercial_leads ORDER BY created_at DESC LIMIT 500",
    )
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
struct LeadUpdateRequest {
    status: Option<String>,
    notes: Option<String>,
}

async fn platform_update_lead(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    Path(lead_id): Path<Uuid>,
    Json(request): Json<LeadUpdateRequest>,
) -> Result<Json<Value>, ApiError> {
    if let Some(status) = &request.status {
        if !matches!(
            status.as_str(),
            "new" | "contacted" | "qualified" | "won" | "lost"
        ) {
            return Err(error(StatusCode::BAD_REQUEST, "status de lead inválido"));
        }
    }
    let changed = sqlx::query(
        "UPDATE commercial_leads SET status=COALESCE($2,status),notes=COALESCE($3,notes) WHERE id=$1",
    )
    .bind(lead_id)
    .bind(request.status.as_deref())
    .bind(request.notes.as_deref().map(str::trim))
    .execute(&state.db)
    .await
    .map_err(internal)?
    .rows_affected();
    if changed == 0 {
        return Err(error(StatusCode::NOT_FOUND, "lead não encontrado"));
    }
    platform_audit(
        &state.db,
        auth.admin_id,
        "commercial_lead.updated",
        None,
        serde_json::json!({"lead_id": lead_id, "status": request.status}),
    )
    .await;
    Ok(Json(serde_json::json!({"lead_id": lead_id, "updated": true})))
}

async fn platform_flags(
    State(state): State<Arc<AppState>>,
    _auth: PlatformAuth,
) -> Result<Json<Vec<Value>>, ApiError> {
    let rows = sqlx::query_as::<_, (String, String, String, bool, bool)>(
        "SELECT key,name,description,requires_homologation,active FROM platform_feature_flags ORDER BY key",
    )
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;
    Ok(Json(
        rows.into_iter()
            .map(|row| {
                serde_json::json!({
                    "key": row.0,
                    "name": row.1,
                    "description": row.2,
                    "requires_homologation": row.3,
                    "active": row.4,
                })
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
struct FlagUpdateRequest {
    active: bool,
}

async fn platform_update_flag(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    Path(key): Path<String>,
    Json(request): Json<FlagUpdateRequest>,
) -> Result<Json<Value>, ApiError> {
    let changed = sqlx::query("UPDATE platform_feature_flags SET active=$2 WHERE key=$1")
        .bind(&key)
        .bind(request.active)
        .execute(&state.db)
        .await
        .map_err(internal)?
        .rows_affected();
    if changed == 0 {
        return Err(error(StatusCode::NOT_FOUND, "feature flag não encontrada"));
    }
    platform_audit(
        &state.db,
        auth.admin_id,
        "feature_flag.updated",
        None,
        serde_json::json!({"key": key, "active": request.active}),
    )
    .await;
    Ok(Json(serde_json::json!({"key": key, "active": request.active})))
}

async fn platform_tenant_flags(
    State(state): State<Arc<AppState>>,
    _auth: PlatformAuth,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<Vec<Value>>, ApiError> {
    let rows = sqlx::query_as::<_, (String, String, String, bool, bool, bool)>(
        "SELECT f.key,f.name,f.description,f.requires_homologation,f.active,COALESCE(tf.enabled,false) FROM platform_feature_flags f LEFT JOIN tenant_feature_flags tf ON tf.key=f.key AND tf.tenant_id=$1 ORDER BY f.key",
    )
    .bind(tenant_id)
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;
    Ok(Json(
        rows.into_iter()
            .map(|row| {
                serde_json::json!({
                    "key": row.0,
                    "name": row.1,
                    "description": row.2,
                    "requires_homologation": row.3,
                    "active": row.4,
                    "enabled": row.5,
                })
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
struct TenantFlagUpdateRequest {
    enabled: bool,
}

async fn platform_set_tenant_flag(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    Path((tenant_id, key)): Path<(Uuid, String)>,
    Json(request): Json<TenantFlagUpdateRequest>,
) -> Result<Json<Value>, ApiError> {
    let flag_active = sqlx::query_scalar::<_, bool>(
        "SELECT active FROM platform_feature_flags WHERE key=$1",
    )
    .bind(&key)
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?;
    let Some(flag_active) = flag_active else {
        return Err(error(StatusCode::NOT_FOUND, "feature flag não encontrada"));
    };
    if request.enabled && !flag_active {
        return Err(error(
            StatusCode::CONFLICT,
            "feature flag desativada na plataforma",
        ));
    }
    sqlx::query(
        "INSERT INTO tenant_feature_flags(tenant_id,key,enabled,updated_at) VALUES($1,$2,$3,now()) ON CONFLICT(tenant_id,key) DO UPDATE SET enabled=excluded.enabled,updated_at=now()",
    )
    .bind(tenant_id)
    .bind(&key)
    .bind(request.enabled)
    .execute(&state.db)
    .await
    .map_err(internal)?;
    platform_audit(
        &state.db,
        auth.admin_id,
        "tenant_feature_flag.updated",
        Some(tenant_id),
        serde_json::json!({"key": key, "enabled": request.enabled}),
    )
    .await;
    Ok(Json(
        serde_json::json!({"tenant_id": tenant_id, "key": key, "enabled": request.enabled}),
    ))
}

/// Abre uma sessão do painel da loja para suporte. Fica registrado na auditoria
/// e expira em 2 horas.
async fn platform_impersonate_tenant(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    Path(tenant_id): Path<Uuid>,
) -> Result<(HeaderMap, Json<Value>), ApiError> {
    let user = sqlx::query_as::<_, (Uuid, String, Option<String>)>(
        "SELECT u.id,u.email,u.display_name FROM tenant_users u JOIN tenants t ON t.id=u.tenant_id WHERE u.tenant_id=$1 AND u.role='owner' AND u.active=true AND u.revoked_at IS NULL AND t.archived_at IS NULL ORDER BY u.created_at LIMIT 1",
    )
    .bind(tenant_id)
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?
    .ok_or_else(|| {
        error(
            StatusCode::NOT_FOUND,
            "loja arquivada ou sem owner ativo para impersonar",
        )
    })?;
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    sqlx::query("INSERT INTO tenant_sessions(token_hash,user_id,expires_at) VALUES($1,$2,now()+interval '2 hours')")
        .bind(hash_key(&token))
        .bind(user.0)
        .execute(&state.db)
        .await
        .map_err(internal)?;
    platform_audit(
        &state.db,
        auth.admin_id,
        "tenant.impersonated",
        Some(tenant_id),
        serde_json::json!({"user_email": user.1}),
    )
    .await;
    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, tenant_session_cookie(&token)?);
    Ok((
        headers,
        Json(serde_json::json!({
            "tenant_id": tenant_id,
            "email": user.1,
            "display_name": user.2,
            "expires_in_seconds": 7200
        })),
    ))
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn csv_response(filename: &str, header: &str, rows: Vec<String>) -> impl IntoResponse {
    let body = format!("{header}\n{}\n", rows.join("\n"));
    (
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        body,
    )
}

async fn platform_export_tenants(
    State(state): State<Arc<AppState>>,
    _auth: PlatformAuth,
) -> Result<impl IntoResponse, ApiError> {
    let rows = sqlx::query_as::<_, PlatformTenantOut>(TENANT_LIST_SQL)
        .bind(true)
        .fetch_all(&state.db)
        .await
        .map_err(internal)?;
    let lines = rows
        .into_iter()
        .map(|row| {
            [
                row.name,
                row.slug.unwrap_or_default(),
                row.owner_email.unwrap_or_default(),
                row.plan.unwrap_or_default(),
                row.subscription_status.unwrap_or_default(),
                row.billing_mode.unwrap_or_default(),
                row.asaas_customer_id.unwrap_or_default(),
                row.units.to_string(),
                row.terminals.to_string(),
                row.online_terminals.to_string(),
                row.mrr_brl_cents.to_string(),
                row.archived_at
                    .map(|value| value.to_rfc3339())
                    .unwrap_or_default(),
                row.created_at.to_rfc3339(),
            ]
            .iter()
            .map(|value| csv_field(value))
            .collect::<Vec<_>>()
            .join(",")
        })
        .collect();
    Ok(csv_response(
        "commercectrl-tenants.csv",
        "nome,slug,owner_email,plano,status,billing_mode,asaas_customer_id,unidades,terminais,online,mrr_cents,arquivado_em,criado_em",
        lines,
    ))
}

async fn platform_export_payments(
    State(state): State<Arc<AppState>>,
    _auth: PlatformAuth,
) -> Result<impl IntoResponse, ApiError> {
    let rows = sqlx::query_as::<_, (String, Uuid, i64, String, Option<String>, chrono::DateTime<chrono::Utc>)>(
        "SELECT t.name,pi.id,pi.amount_brl_cents,pi.status,pi.provider_id,pi.created_at FROM payment_intents pi JOIN tenants t ON t.id=pi.tenant_id WHERE pi.provider='asaas' ORDER BY pi.created_at DESC LIMIT 5000",
    )
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;
    let lines = rows
        .into_iter()
        .map(|row| {
            [
                row.0,
                row.1.to_string(),
                row.2.to_string(),
                row.3,
                row.4.unwrap_or_default(),
                row.5.to_rfc3339(),
            ]
            .iter()
            .map(|value| csv_field(value))
            .collect::<Vec<_>>()
            .join(",")
        })
        .collect();
    Ok(csv_response(
        "commercectrl-cobrancas.csv",
        "tenant,payment_id,valor_cents,status,asaas_id,criado_em",
        lines,
    ))
}

async fn platform_export_audit(
    State(state): State<Arc<AppState>>,
    _auth: PlatformAuth,
) -> Result<impl IntoResponse, ApiError> {
    let rows = sqlx::query_as::<_, (i64, String, Option<String>, Option<String>, String, chrono::DateTime<chrono::Utc>)>(
        "SELECT e.id,e.action,a.email,t.name,e.metadata::text,e.created_at FROM platform_audit_events e LEFT JOIN platform_admins a ON a.id=e.admin_id LEFT JOIN tenants t ON t.id=e.tenant_id ORDER BY e.created_at DESC LIMIT 5000",
    )
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;
    let lines = rows
        .into_iter()
        .map(|row| {
            [
                row.0.to_string(),
                row.1,
                row.2.unwrap_or_default(),
                row.3.unwrap_or_default(),
                row.4,
                row.5.to_rfc3339(),
            ]
            .iter()
            .map(|value| csv_field(value))
            .collect::<Vec<_>>()
            .join(",")
        })
        .collect();
    Ok(csv_response(
        "commercectrl-auditoria.csv",
        "id,acao,admin,tenant,metadata,criado_em",
        lines,
    ))
}

async fn commercial_lead(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CommercialLeadRequest>,
) -> Result<Json<Value>, ApiError> {
    let store_name = request.store_name.trim();
    if store_name.len() < 2
        || store_name.len() > 120
        || !matches!(request.modality.as_str(), "purchase" | "loan")
    {
        return Err(error(StatusCode::BAD_REQUEST, "dados comerciais inválidos"));
    }
    if let Some(code) = &request.plan_code {
        let valid = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM platform_plan_catalog WHERE code=$1 AND active=true)",
        )
        .bind(code)
        .fetch_one(&state.db)
        .await
        .map_err(internal)?;
        if !valid {
            return Err(error(StatusCode::BAD_REQUEST, "plano inválido"));
        }
    }
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO commercial_leads(id,store_name,contact_name,email,phone,city,plan_code,modality,details) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)").bind(id).bind(store_name).bind(request.contact_name).bind(request.email).bind(request.phone).bind(request.city).bind(request.plan_code).bind(request.modality).bind(request.details).execute(&state.db).await.map_err(internal)?;
    Ok(Json(serde_json::json!({"id":id,"received":true})))
}
async fn platform_commercial_catalog(
    State(state): State<Arc<AppState>>,
    _auth: PlatformAuth,
) -> Result<Json<Value>, ApiError> {
    let plans=sqlx::query_as::<_,CommercialPlanOut>("SELECT code,name,monthly_brl_cents,description,fiscal_eligible,premium_pdv,active FROM platform_plan_catalog ORDER BY monthly_brl_cents").fetch_all(&state.db).await.map_err(internal)?;
    let equipment=sqlx::query_as::<_,EquipmentOut>("SELECT id,sku,name,category,sale_brl_cents,rental_brl_cents,source_url,active FROM platform_equipment_catalog ORDER BY name").fetch_all(&state.db).await.map_err(internal)?;
    let flags=sqlx::query_as::<_,(String,String,String,bool,bool)>("SELECT key,name,description,requires_homologation,active FROM platform_feature_flags ORDER BY key").fetch_all(&state.db).await.map_err(internal)?;
    Ok(Json(
        serde_json::json!({"plans":plans,"equipment":equipment,"flags":flags.into_iter().map(|f|serde_json::json!({"key":f.0,"name":f.1,"description":f.2,"requires_homologation":f.3,"active":f.4})).collect::<Vec<_>>()}),
    ))
}
async fn platform_create_equipment(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    Json(input): Json<EquipmentInput>,
) -> Result<Json<EquipmentOut>, ApiError> {
    if input.sku.trim().is_empty()
        || input.name.trim().is_empty()
        || input.category.trim().is_empty()
        || input.sale_brl_cents.is_some_and(|v| v < 0)
        || input.rental_brl_cents.is_some_and(|v| v < 0)
    {
        return Err(error(StatusCode::BAD_REQUEST, "equipamento inválido"));
    }
    let item=sqlx::query_as::<_,EquipmentOut>("INSERT INTO platform_equipment_catalog(id,sku,name,category,sale_brl_cents,rental_brl_cents,source_url,active) VALUES($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id,sku,name,category,sale_brl_cents,rental_brl_cents,source_url,active").bind(Uuid::new_v4()).bind(input.sku.trim()).bind(input.name.trim()).bind(input.category.trim()).bind(input.sale_brl_cents).bind(input.rental_brl_cents).bind(input.source_url).bind(input.active).fetch_one(&state.db).await.map_err(internal)?;
    platform_audit(
        &state.db,
        auth.admin_id,
        "equipment.created",
        None,
        serde_json::json!({"equipment_id":item.id,"sku":item.sku}),
    )
    .await;
    Ok(Json(item))
}

#[derive(Debug, Deserialize)]
struct TenantSubscriptionPaymentRequest {
    amount_brl_cents: i64,
    billing_type: String,
    description: Option<String>,
    /// Período de assinatura liberado após a confirmação (dias).
    validity_days: Option<i64>,
    /// Vencimento da cobrança no Asaas; padrão igual a validity_days.
    due_days: Option<i64>,
}

#[derive(Debug, Serialize)]
struct TenantPaymentIntentOut {
    id: Uuid,
    external_reference: String,
    amount_brl_cents: i64,
    status: String,
    qr_payload: Option<String>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    checkout_url: Option<String>,
}

async fn platform_create_tenant(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    Json(request): Json<PlatformTenantCreateRequest>,
) -> Result<Json<PlatformTenantCreated>, ApiError> {
    Ok(Json(
        provision_tenant(&state, Some(&auth), &request, None).await?,
    ))
}

/// Identidade visual opcional enviada no autoatendimento.
#[derive(Debug, Default, Clone)]
struct SignupBranding {
    logo_data_url: Option<String>,
    primary_color: Option<String>,
}

/// Cria tenant, unidade, assinatura e usuário owner. Usado pelo SuperAdmin e
/// pelo autoatendimento público (com auditoria apenas no primeiro caso).
async fn provision_tenant(
    state: &Arc<AppState>,
    admin: Option<&PlatformAuth>,
    request: &PlatformTenantCreateRequest,
    branding: Option<&SignupBranding>,
) -> Result<PlatformTenantCreated, ApiError> {
    let actor = admin
        .map(|auth| auth.email.as_str())
        .unwrap_or("self-signup");
    let name = request.name.trim();
    let unit_name = request.unit_name.trim();
    let billing_mode = request
        .billing_mode
        .as_deref()
        .unwrap_or("paid")
        .trim()
        .to_lowercase();
    if name.len() < 2
        || name.len() > 120
        || unit_name.len() < 2
        || unit_name.len() > 120
        || !matches!(
            request.plan.as_str(),
            "starter" | "fiscal" | "premium" | "essencial" | "profissional" | "rede"
        )
        || !(0..=365).contains(&request.trial_days)
        || !matches!(billing_mode.as_str(), "paid" | "courtesy")
    {
        return Err(error(StatusCode::BAD_REQUEST, "dados do tenant inválidos"));
    }

    let now = chrono::Utc::now();
    let max_courtesy = now + chrono::Duration::days(730);
    let courtesy_until = if billing_mode == "courtesy" {
        let until = match request.courtesy_until {
            Some(value) => value,
            None => {
                let months = request.courtesy_months.unwrap_or(1);
                if !(1..=24).contains(&months) {
                    return Err(error(
                        StatusCode::BAD_REQUEST,
                        "período de cortesia deve ter entre 1 e 24 meses",
                    ));
                }
                now + chrono::Duration::days(months * 30)
            }
        };
        if until <= now || until > max_courtesy {
            return Err(error(
                StatusCode::BAD_REQUEST,
                "a cortesia deve terminar entre hoje e 2 anos",
            ));
        }
        Some(until)
    } else {
        None
    };
    let courtesy_note = request
        .courtesy_note
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(200).collect::<String>());

    let tenant_id = Uuid::new_v4();
    let unit_id = Uuid::new_v4();
    let trial_ends_at = now + chrono::Duration::days(request.trial_days);
    let period_ends_at = courtesy_until.unwrap_or(trial_ends_at);
    let slug = unique_slug(&state.db, name).await.map_err(internal)?;
    let owner_email = request
        .email
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_lowercase)
        .unwrap_or_else(|| format!("dono@{slug}.commercectrl.local"));
    let chosen_password = request
        .password
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(password) = chosen_password {
        if password.len() < 8 || password.len() > 128 {
            return Err(error(
                StatusCode::BAD_REQUEST,
                "a senha deve ter entre 8 e 128 caracteres",
            ));
        }
    }
    let owner_password = chosen_password
        .map(str::to_string)
        .unwrap_or_else(temporary_password);
    let must_change_password = chosen_password.is_none();
    let owner_password_hash = password_hash(&owner_password)?;
    let owner_key = random_secret("ok_");

    let mut asaas_customer_id: Option<String> = None;
    if billing_mode == "paid" && request.create_asaas_customer.unwrap_or(false) {
        match create_asaas_customer(
            &request.name,
            request.email.as_deref(),
            request.cnpj_cpf.as_deref(),
            request.phone.as_deref(),
        )
        .await
        {
            Ok(id) => asaas_customer_id = Some(id),
            Err(e) => {
                warn!(actor, tenant=%tenant_id, "falha ao criar customer Asaas: {:?}", e)
            }
        }
    }

    let status = if billing_mode == "courtesy" {
        "active"
    } else if request.trial_days > 0 {
        "trial"
    } else {
        "active"
    };
    let subscription_trial_ends_at = if billing_mode == "courtesy" {
        None
    } else {
        Some(trial_ends_at)
    };

    let mut tx = state.db.begin().await.map_err(internal)?;
    sqlx::query("INSERT INTO tenants(id,name,slug,cnpj_cpf,phone) VALUES($1,$2,$3,$4,$5)")
        .bind(tenant_id)
        .bind(name)
        .bind(&slug)
        .bind(request.cnpj_cpf.as_deref().map(str::trim).filter(|value| !value.is_empty()))
        .bind(request.phone.as_deref().map(str::trim).filter(|value| !value.is_empty()))
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    sqlx::query("INSERT INTO units(id,tenant_id,name,code) VALUES($1,$2,$3,'MATRIZ')")
        .bind(unit_id)
        .bind(tenant_id)
        .bind(unit_name)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    let brand_logo = branding.and_then(|value| value.logo_data_url.as_deref());
    let brand_color = branding
        .and_then(|value| value.primary_color.as_deref())
        .filter(|color| color.len() == 7 && color.starts_with('#'))
        .unwrap_or("#0D5C3A");
    sqlx::query(
        "INSERT INTO tenant_branding(tenant_id,display_name,logo_data_url,primary_color) VALUES($1,$2,$3,$4) ON CONFLICT(tenant_id) DO NOTHING",
    )
    .bind(tenant_id)
    .bind(name)
    .bind(brand_logo)
    .bind(brand_color)
    .execute(&mut *tx)
    .await
    .map_err(internal)?;
    sqlx::query(
        "INSERT INTO tenant_admins(tenant_id,email,role,api_key_hash) VALUES($1,$2,'owner',$3)",
    )
    .bind(tenant_id)
    .bind(&owner_email)
    .bind(hash_key(&owner_key))
    .execute(&mut *tx)
    .await
    .map_err(internal)?;
    sqlx::query("INSERT INTO tenant_users(tenant_id,email,password_hash,role,active,must_change_password,display_name,source) VALUES($1,$2,$3,'owner',true,$4,$5,'cloud')")
        .bind(tenant_id)
        .bind(&owner_email)
        .bind(&owner_password_hash)
        .bind(must_change_password)
        .bind(format!("Administrador {name}"))
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    sqlx::query("INSERT INTO tenant_subscriptions(tenant_id,plan,status,trial_ends_at,current_period_ends_at,asaas_customer_id,billing_mode,courtesy_until,courtesy_note) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)")
        .bind(tenant_id)
        .bind(&request.plan)
        .bind(status)
        .bind(subscription_trial_ends_at)
        .bind(period_ends_at)
        .bind(&asaas_customer_id)
        .bind(&billing_mode)
        .bind(courtesy_until)
        .bind(&courtesy_note)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    if let Some(auth) = admin {
        platform_audit(
            &state.db,
            auth.admin_id,
            "tenant.created",
            Some(tenant_id),
            serde_json::json!({
                "plan": request.plan,
                "trial_days": request.trial_days,
                "unit_id": unit_id,
                "asaas_customer_id": asaas_customer_id,
                "billing_mode": billing_mode,
                "courtesy_until": courtesy_until,
                "owner_email": owner_email
            }),
        )
        .await;
    }
    info!(actor, tenant=%tenant_id, billing_mode=%billing_mode, "tenant provisionado");
    Ok(PlatformTenantCreated {
        tenant_id,
        unit_id,
        name: name.into(),
        slug,
        trial_ends_at,
        asaas_customer_id,
        billing_mode,
        courtesy_until,
        owner_email,
        owner_password,
        owner_key,
    })
}

/// Autoatendimento público: cria a loja, o owner e a assinatura, e já tenta
/// gerar a cobrança do primeiro mês. A conta nasce em teste até o pagamento.
async fn public_signup(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(request): Json<SignupRequest>,
) -> Result<Json<SignupCreated>, ApiError> {
    let ip = client_ip(&headers, addr);
    let recent = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)::bigint FROM login_attempts WHERE ip=$1 AND email='self-signup' AND attempted_at>now()-interval '1 hour'",
    )
    .bind(&ip)
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;
    if recent >= 10 {
        return Err(error(
            StatusCode::TOO_MANY_REQUESTS,
            "muitos cadastros deste endereço; tente novamente em 1 hora",
        ));
    }
    let _ =
        sqlx::query("INSERT INTO login_attempts(email,ip,succeeded) VALUES('self-signup',$1,true)")
            .bind(&ip)
            .execute(&state.db)
            .await;

    let name = request.name.trim();
    let unit_name = request.unit_name.trim();
    let email = request.email.trim().to_lowercase();
    let valid_email = email.len() <= 200
        && email.split_once('@').is_some_and(|(user, domain)| {
            !user.is_empty() && domain.contains('.') && !domain.starts_with('.')
        });
    if name.len() < 2
        || name.len() > 120
        || unit_name.len() < 2
        || unit_name.len() > 120
        || !valid_email
    {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "dados do cadastro inválidos",
        ));
    }
    let signup_password = request.password.trim();
    if signup_password.len() < 8 || signup_password.len() > 128 {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "a senha deve ter entre 8 e 128 caracteres",
        ));
    }
    let billing_type = request
        .billing_type
        .as_deref()
        .unwrap_or("PIX")
        .trim()
        .to_uppercase();
    if !matches!(billing_type.as_str(), "PIX" | "CREDIT_CARD" | "BOLETO") {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "forma de pagamento inválida",
        ));
    }
    if let Some(logo) = request.logo_data_url.as_deref() {
        if !logo.starts_with("data:image/") || logo.len() > 2_800_000 {
            return Err(error(
                StatusCode::BAD_REQUEST,
                "logo inválida ou maior que 2 MB",
            ));
        }
    }
    let plan_price = sqlx::query_scalar::<_, i32>(
        "SELECT monthly_brl_cents FROM platform_plan_catalog WHERE code=$1 AND active=true",
    )
    .bind(&request.plan)
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?
    .ok_or_else(|| error(StatusCode::BAD_REQUEST, "plano inválido"))?;

    let trial_days = env::var("SIGNUP_TRIAL_DAYS")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(7)
        .clamp(0, 30);
    let create = PlatformTenantCreateRequest {
        name: name.to_string(),
        unit_name: unit_name.to_string(),
        email: Some(email),
        password: Some(signup_password.to_string()),
        cnpj_cpf: request
            .cnpj_cpf
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        phone: request
            .phone
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        plan: request.plan.clone(),
        trial_days,
        create_asaas_customer: Some(true),
        billing_mode: Some("paid".into()),
        courtesy_months: None,
        courtesy_until: None,
        courtesy_note: None,
    };
    let branding = SignupBranding {
        logo_data_url: request.logo_data_url.clone(),
        primary_color: request.primary_color.clone(),
    };
    let created = provision_tenant(&state, None, &create, Some(&branding)).await?;

    let mut payment = None;
    let mut payment_error = None;
    if plan_price > 0 {
        if created.asaas_customer_id.is_some() {
            let payment_request = TenantSubscriptionPaymentRequest {
                amount_brl_cents: plan_price as i64,
                billing_type,
                description: Some(format!(
                    "Assinatura CommerceCTRL - {} ({})",
                    name, request.plan
                )),
                validity_days: Some(30),
                due_days: Some(3),
            };
            match create_subscription_payment(&state, created.tenant_id, &payment_request).await {
                Ok(value) => payment = Some(value),
                Err((_, Json(body))) => {
                    warn!(tenant=%created.tenant_id, error=%body, "cobrança do autoatendimento falhou");
                    payment_error = body
                        .get("error")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                }
            }
        } else {
            payment_error =
                Some("Pagamento indisponível no momento; sua conta está em teste.".into());
        }
    }
    info!(tenant=%created.tenant_id, plan=%create.plan, "tenant criado por autoatendimento");
    Ok(Json(SignupCreated {
        tenant_id: created.tenant_id,
        unit_id: created.unit_id,
        name: created.name,
        slug: created.slug,
        trial_ends_at: created.trial_ends_at,
        billing_mode: created.billing_mode,
        courtesy_until: created.courtesy_until,
        owner_email: created.owner_email,
        payment,
        payment_error,
    }))
}

#[derive(Debug, Serialize)]
struct AsaasCustomerRequest {
    name: String,
    email: Option<String>,
    cpf_cnpj: Option<String>,
    phone: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AsaasCustomerResponse {
    id: String,
}

async fn create_asaas_customer(
    name: &str,
    email: Option<&str>,
    cpf_cnpj: Option<&str>,
    phone: Option<&str>,
) -> Result<String, ApiError> {
    let api_key = env::var("ASAAS_API_KEY").unwrap_or_default();
    let base_url =
        env::var("ASAAS_BASE_URL").unwrap_or_else(|_| "https://api-sandbox.asaas.com/v3".into());
    if api_key.is_empty() {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "Asaas API key não configurada",
        ));
    }
    let client = reqwest::Client::new();
    let payload = AsaasCustomerRequest {
        name: name.to_string(),
        email: email.map(|s| s.to_string()),
        cpf_cnpj: cpf_cnpj.map(|s| s.to_string()),
        phone: phone.map(|s| s.to_string()),
    };
    let resp = client
        .post(format!("{}/customers", base_url))
        .header("access_token", &api_key)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| error(StatusCode::BAD_GATEWAY, format!("Asaas indisponível: {e}")))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(error(
            StatusCode::BAD_GATEWAY,
            format!("Asaas recusou ({status}): {text}"),
        ));
    }
    let created: AsaasCustomerResponse = resp.json().await.map_err(internal)?;
    Ok(created.id)
}

async fn platform_create_asaas_customer(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<PlatformTenantCreateRequest>,
) -> Result<Json<Value>, ApiError> {
    let exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM tenants WHERE id=$1)")
        .bind(tenant_id)
        .fetch_one(&state.db)
        .await
        .map_err(internal)?;
    if !exists {
        return Err(error(StatusCode::NOT_FOUND, "tenant não encontrado"));
    }
    let id = create_asaas_customer(
        &request.name,
        request.email.as_deref(),
        request.cnpj_cpf.as_deref(),
        request.phone.as_deref(),
    )
    .await?;
    sqlx::query(
        "UPDATE tenant_subscriptions SET asaas_customer_id=$1, updated_at=now() WHERE tenant_id=$2",
    )
    .bind(&id)
    .bind(tenant_id)
    .execute(&state.db)
    .await
    .map_err(internal)?;
    platform_audit(
        &state.db,
        auth.admin_id,
        "asaas_customer.created",
        Some(tenant_id),
        serde_json::json!({"asaas_customer_id": id}),
    )
    .await;
    Ok(Json(
        serde_json::json!({"tenant_id": tenant_id, "asaas_customer_id": id}),
    ))
}

async fn platform_create_subscription_payment(
    State(state): State<Arc<AppState>>,
    _auth: PlatformAuth,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<TenantSubscriptionPaymentRequest>,
) -> Result<Json<TenantPaymentIntentOut>, ApiError> {
    Ok(Json(
        create_subscription_payment(&state, tenant_id, &request).await?,
    ))
}

async fn create_subscription_payment(
    state: &Arc<AppState>,
    tenant_id: Uuid,
    request: &TenantSubscriptionPaymentRequest,
) -> Result<TenantPaymentIntentOut, ApiError> {
    let row = sqlx::query_as::<_, (Uuid, Option<String>)>(
        "SELECT tenant_id, asaas_customer_id FROM tenant_subscriptions WHERE tenant_id=$1",
    )
    .bind(tenant_id)
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?
    .ok_or_else(|| error(StatusCode::NOT_FOUND, "tenant not found"))?;
    let asaas_customer_id = match row.1 {
        Some(ref c) if !c.is_empty() => c.clone(),
        _ => {
            return Err(error(
                StatusCode::BAD_REQUEST,
                "Asaas customer not registered",
            ))
        }
    };

    let existing = sqlx::query_as::<_, PaymentIntentOut>(
        "SELECT id,external_reference,amount_brl_cents,status,qr_payload,expires_at FROM payment_intents WHERE tenant_id=$1 AND provider='asaas' AND status IN ('creating','pending','received','confirmed') ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&tenant_id)
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?;

    if let Some(existing) = existing {
        if existing.status == "received" || existing.status == "confirmed" {
            return Err(error(StatusCode::CONFLICT, "payment already approved"));
        }
        return Ok(TenantPaymentIntentOut {
            id: existing.id,
            external_reference: existing.external_reference,
            amount_brl_cents: existing.amount_brl_cents,
            status: existing.status,
            qr_payload: existing.qr_payload,
            expires_at: existing.expires_at,
            checkout_url: None,
        });
    }

    let api_key = env::var("ASAAS_API_KEY").unwrap_or_default();
    let base_url =
        env::var("ASAAS_BASE_URL").unwrap_or_else(|_| "https://api-sandbox.asaas.com/v3".into());
    if api_key.is_empty() {
        return Err(error(
            StatusCode::BAD_GATEWAY,
            "Asaas API key not configured",
        ));
    }

    let external_reference = format!("sub-{}-{}", tenant_id, Uuid::new_v4().simple());
    let id = Uuid::new_v4();
    let validity_days = request.validity_days.unwrap_or(30).clamp(1, 730);
    let due_days = request.due_days.unwrap_or(validity_days).clamp(1, 730);
    let expires_at = chrono::Utc::now() + chrono::Duration::days(due_days);

    sqlx::query(
        "INSERT INTO payment_intents(id,tenant_id,unit_id,terminal_id,provider,external_reference,amount_brl_cents,status,raw,created_at) VALUES($1,$2,$3,$4,'asaas',$5,$6,'creating',$7,now())",
    )
    .bind(id)
    .bind(tenant_id)
    .bind(Uuid::nil())
    .bind(Uuid::nil())
    .bind(&external_reference)
    .bind(request.amount_brl_cents)
    .bind(serde_json::json!({"validity_days": validity_days, "due_days": due_days}))
    .execute(&state.db)
    .await
    .map_err(internal)?;

    let client = reqwest::Client::new();
    let created_response = client
        .post(format!("{base_url}/payments"))
        .header("access_token", &api_key)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "customer": asaas_customer_id,
            "billingType": request.billing_type,
            "value": request.amount_brl_cents as f64 / 100.0,
            "dueDate": expires_at.date_naive().format("%Y-%m-%d").to_string(),
            "description": request.description.clone().unwrap_or_else(|| format!("Assinatura CommerceCTRL - {}", row.1.clone().unwrap_or_default())),
            "externalReference": &external_reference,
        }))
        .send()
        .await
        .map_err(|e| error(StatusCode::BAD_GATEWAY, format!("Asaas indisponível: {e}")))?;

    if !created_response.status().is_success() {
        let status = created_response.status();
        let body = created_response.text().await.unwrap_or_default();
        sqlx::query(
            "UPDATE payment_intents SET status='failed',raw=$2,updated_at=now() WHERE id=$1",
        )
        .bind(id)
        .bind(serde_json::json!({"http_status":status.as_u16(),"body":body}))
        .execute(&state.db)
        .await
        .map_err(internal)?;
        return Err(error(
            StatusCode::BAD_GATEWAY,
            format!("Asaas recusou ({status}): {body}"),
        ));
    }

    let created: serde_json::Value = created_response.json().await.map_err(internal)?;

    let qr_payload = if request.billing_type == "PIX" {
        let qr_response = client
            .get(format!("{base_url}/payments/{}/pixQrCode", created["id"]))
            .header("access_token", &api_key)
            .send()
            .await
            .map_err(|e| error(StatusCode::BAD_GATEWAY, format!("QR PIX indisponível: {e}")))?;
        if qr_response.status().is_success() {
            let qr: serde_json::Value = qr_response.json().await.map_err(internal)?;
            qr.get("payload")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        } else {
            None
        }
    } else {
        None
    };

    let expires_at_dt = expires_at;
    sqlx::query(
        "UPDATE payment_intents SET provider_id=$2,status=$3,qr_payload=$4,expires_at=$5,raw=$6,updated_at=now() WHERE id=$1",
    )
    .bind(id)
    .bind(&created["id"])
    .bind("pending")
    .bind(&qr_payload)
    .bind(&expires_at_dt)
    .bind(serde_json::json!({"asaas_status": created.get("status").and_then(|v| v.as_str()).unwrap_or("unknown")}))
    .execute(&state.db)
    .await
    .map_err(internal)?;

    let result = sqlx::query_as::<_, PaymentIntentOut>(
        "SELECT id,external_reference,amount_brl_cents,status,qr_payload,expires_at FROM payment_intents WHERE id=$1",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;

    Ok(TenantPaymentIntentOut {
        id: result.id,
        external_reference: result.external_reference,
        amount_brl_cents: result.amount_brl_cents,
        status: result.status,
        qr_payload: result.qr_payload,
        expires_at: result.expires_at,
        checkout_url: None,
    })
}

async fn platform_payment_status(
    State(state): State<Arc<AppState>>,
    _auth: PlatformAuth,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<TenantPaymentIntentOut>, ApiError> {
    let result = sqlx::query_as::<_, PaymentIntentOut>(
        "SELECT id,external_reference,amount_brl_cents,status,qr_payload,expires_at FROM payment_intents WHERE tenant_id=$1 AND provider='asaas' ORDER BY created_at DESC LIMIT 1",
    )
    .bind(tenant_id)
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?
    .ok_or_else(|| error(StatusCode::NOT_FOUND, "nenhum pagamento encontrado para este tenant"))?;
    Ok(Json(TenantPaymentIntentOut {
        id: result.id,
        external_reference: result.external_reference,
        amount_brl_cents: result.amount_brl_cents,
        status: result.status,
        qr_payload: result.qr_payload,
        expires_at: result.expires_at,
        checkout_url: None,
    }))
}

async fn platform_create_activation_code(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    Path((tenant_id, unit_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<PlatformActivationCodeRequest>,
) -> Result<Json<PlatformActivationCodeCreated>, ApiError> {
    if !(1..=30).contains(&request.valid_days) || !(1..=50).contains(&request.max_uses) {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "validade ou quantidade de ativações inválida",
        ));
    }
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM units WHERE tenant_id=$1 AND id=$2)",
    )
    .bind(tenant_id)
    .bind(unit_id)
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;
    if !exists {
        return Err(error(StatusCode::NOT_FOUND, "unidade não encontrada"));
    }
    let code = format!(
        "CC-{}",
        Uuid::new_v4().simple().to_string()[..12].to_uppercase()
    );
    let activation_id = Uuid::new_v4();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(request.valid_days);
    sqlx::query("INSERT INTO platform_activation_codes(id,tenant_id,unit_id,code_hash,expires_at,max_uses,created_by) VALUES($1,$2,$3,$4,$5,$6,$7)").bind(activation_id).bind(tenant_id).bind(unit_id).bind(hash_key(&code)).bind(expires_at).bind(request.max_uses).bind(auth.admin_id).execute(&state.db).await.map_err(internal)?;
    platform_audit(&state.db, auth.admin_id, "activation_code.created", Some(tenant_id), serde_json::json!({"activation_code_id":activation_id,"unit_id":unit_id,"valid_days":request.valid_days,"max_uses":request.max_uses})).await;
    Ok(Json(PlatformActivationCodeCreated {
        id: activation_id,
        code,
        expires_at,
        max_uses: request.max_uses,
    }))
}

async fn platform_suspend_tenant(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let mut tx = state.db.begin().await.map_err(internal)?;
    let changed = sqlx::query("UPDATE tenant_subscriptions SET status='suspended',updated_at=now() WHERE tenant_id=$1 AND status<>'cancelled'")
        .bind(tenant_id).execute(&mut *tx).await.map_err(internal)?.rows_affected();
    if changed == 0 {
        return Err(error(
            StatusCode::NOT_FOUND,
            "assinatura do tenant não encontrada",
        ));
    }
    sqlx::query("UPDATE terminals SET active=false WHERE tenant_id=$1")
        .bind(tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    sqlx::query("UPDATE platform_activation_codes SET revoked_at=now() WHERE tenant_id=$1 AND revoked_at IS NULL").bind(tenant_id).execute(&mut *tx).await.map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    platform_audit(
        &state.db,
        auth.admin_id,
        "tenant.suspended",
        Some(tenant_id),
        serde_json::json!({}),
    )
    .await;
    info!(admin=%auth.email,tenant=%tenant_id,"tenant suspenso pela plataforma");
    Ok(Json(
        serde_json::json!({"tenant_id":tenant_id,"status":"suspended"}),
    ))
}

async fn platform_restore_tenant(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let changed = sqlx::query("UPDATE tenant_subscriptions SET status=CASE WHEN trial_ends_at>now() THEN 'trial' ELSE 'active' END,current_period_ends_at=GREATEST(COALESCE(current_period_ends_at,now()),now())+interval '7 days',updated_at=now() WHERE tenant_id=$1 AND status='suspended'")
        .bind(tenant_id).execute(&state.db).await.map_err(internal)?.rows_affected();
    if changed == 0 {
        return Err(error(
            StatusCode::CONFLICT,
            "tenant não está suspenso ou não foi encontrado",
        ));
    }
    platform_audit(
        &state.db,
        auth.admin_id,
        "tenant.restored",
        Some(tenant_id),
        serde_json::json!({"terminals_require_reactivation":true}),
    )
    .await;
    // Terminais permanecem bloqueados até a reativação consciente no painel: evita
    // reativar uma máquina perdida somente por restaurar a assinatura.
    info!(admin=%auth.email,tenant=%tenant_id,"tenant reativado pela plataforma");
    Ok(Json(
        serde_json::json!({"tenant_id":tenant_id,"status":"active","terminals_require_reactivation":true}),
    ))
}

#[derive(Debug, Deserialize)]
struct CourtesyRequest {
    months: Option<i64>,
    until: Option<chrono::DateTime<chrono::Utc>>,
    note: Option<String>,
    #[serde(default)]
    revoke: bool,
}

/// Concede, estende ou revoga a cortesia de uma loja inteira.
async fn platform_set_courtesy(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<CourtesyRequest>,
) -> Result<Json<Value>, ApiError> {
    let current = sqlx::query_as::<_, (Option<String>, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT s.billing_mode,s.courtesy_until FROM tenants t LEFT JOIN tenant_subscriptions s ON s.tenant_id=t.id WHERE t.id=$1",
    )
    .bind(tenant_id)
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?
    .ok_or_else(|| error(StatusCode::NOT_FOUND, "tenant não encontrado"))?;
    let note = request
        .note
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(200).collect::<String>());

    if request.revoke {
        sqlx::query("UPDATE tenant_subscriptions SET status='suspended',courtesy_until=now(),updated_at=now() WHERE tenant_id=$1")
            .bind(tenant_id)
            .execute(&state.db)
            .await
            .map_err(internal)?;
        sqlx::query("UPDATE licenses SET status='suspended',updated_at=now() WHERE tenant_id=$1")
            .bind(tenant_id)
            .execute(&state.db)
            .await
            .map_err(internal)?;
        platform_audit(
            &state.db,
            auth.admin_id,
            "courtesy.revoked",
            Some(tenant_id),
            serde_json::json!({"note": note}),
        )
        .await;
        info!(admin=%auth.email,tenant=%tenant_id,"cortesia revogada");
        return Ok(Json(serde_json::json!({
            "tenant_id": tenant_id,
            "status": "suspended",
            "billing_mode": "courtesy"
        })));
    }

    let now = chrono::Utc::now();
    let max_courtesy = now + chrono::Duration::days(730);
    // Estender soma ao prazo atual; conceder de novo parte de hoje.
    let base = match current.1 {
        Some(until) if current.0.as_deref() == Some("courtesy") && until > now => until,
        _ => now,
    };
    let until = match request.until {
        Some(value) => value,
        None => {
            let months = request.months.unwrap_or(1);
            if !(1..=24).contains(&months) {
                return Err(error(
                    StatusCode::BAD_REQUEST,
                    "período de cortesia deve ter entre 1 e 24 meses",
                ));
            }
            base + chrono::Duration::days(months * 30)
        }
    };
    if until <= now || until > max_courtesy {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "a cortesia deve terminar entre hoje e 2 anos",
        ));
    }
    let ends_at = until.timestamp();
    let mut tx = state.db.begin().await.map_err(internal)?;
    let changed = sqlx::query(
        "UPDATE tenant_subscriptions SET status='active',billing_mode='courtesy',trial_ends_at=NULL,current_period_ends_at=$2,courtesy_until=$2,courtesy_note=$3,updated_at=now() WHERE tenant_id=$1",
    )
    .bind(tenant_id)
    .bind(until)
    .bind(&note)
    .execute(&mut *tx)
    .await
    .map_err(internal)?
    .rows_affected();
    if changed == 0 {
        return Err(error(
            StatusCode::NOT_FOUND,
            "assinatura do tenant não encontrada",
        ));
    }
    sqlx::query(
        "UPDATE licenses SET status='active',expires_at=$2,grace_until=$3,updated_at=now() WHERE tenant_id=$1",
    )
    .bind(tenant_id)
    .bind(ends_at)
    .bind(ends_at + 7 * 86_400)
    .execute(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    platform_audit(
        &state.db,
        auth.admin_id,
        "courtesy.granted",
        Some(tenant_id),
        serde_json::json!({"until": until, "note": note}),
    )
    .await;
    info!(admin=%auth.email,tenant=%tenant_id,until=%until,"cortesia concedida/estendida");
    Ok(Json(serde_json::json!({
        "tenant_id": tenant_id,
        "status": "active",
        "billing_mode": "courtesy",
        "courtesy_until": until
    })))
}

async fn platform_revoke_activation_code(
    State(state): State<Arc<AppState>>,
    auth: PlatformAuth,
    Path(code_id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let changed = sqlx::query("UPDATE platform_activation_codes SET revoked_at=now() WHERE id=$1 AND revoked_at IS NULL AND uses<max_uses")
        .bind(code_id).execute(&state.db).await.map_err(internal)?.rows_affected();
    if changed == 0 {
        return Err(error(
            StatusCode::NOT_FOUND,
            "código não encontrado, já utilizado ou já revogado",
        ));
    }
    platform_audit(
        &state.db,
        auth.admin_id,
        "activation_code.revoked",
        None,
        serde_json::json!({"activation_code_id":code_id}),
    )
    .await;
    info!(admin=%auth.email,activation_code=%code_id,"código de ativação revogado");
    Ok(Json(
        serde_json::json!({"activation_code_id":code_id,"status":"revoked"}),
    ))
}

async fn claim_activation_code(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ActivationClaimRequest>,
) -> Result<Json<ActivationClaimed>, ApiError> {
    let code = request.code.trim().to_uppercase();
    if !code.starts_with("CC-")
        || code.len() > 64
        || request.installation_id.trim().is_empty()
        || request.terminal_name.trim().len() < 2
        || request.terminal_name.trim().len() > 120
    {
        return Err(error(StatusCode::BAD_REQUEST, "ativação inválida"));
    }
    let mut tx = state.db.begin().await.map_err(internal)?;
    let row=sqlx::query_as::<_,(Uuid,Uuid)>("SELECT tenant_id,unit_id FROM platform_activation_codes WHERE code_hash=$1 AND revoked_at IS NULL AND expires_at>now() AND uses<max_uses FOR UPDATE").bind(hash_key(&code)).fetch_optional(&mut *tx).await.map_err(internal)?.ok_or_else(||error(StatusCode::UNAUTHORIZED,"código inválido ou expirado"))?;
    let subscription = sqlx::query_as::<_, (Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>)>("SELECT current_period_ends_at,trial_ends_at FROM tenant_subscriptions WHERE tenant_id=$1 AND status IN ('trial','active') AND (current_period_ends_at IS NULL OR current_period_ends_at>now())")
        .bind(row.0).fetch_optional(&mut *tx).await.map_err(internal)?;
    let Some((period_ends_at, trial_ends_at)) = subscription else {
        return Err(error(
            StatusCode::PAYMENT_REQUIRED,
            "assinatura não permite ativação",
        ));
    };
    let terminal_id = Uuid::new_v4();
    let terminal_key = format!("ct_{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let (tenant_slug, tenant_name) =
        sqlx::query_as::<_, (String, String)>("SELECT slug,name FROM tenants WHERE id=$1")
            .bind(row.0)
            .fetch_one(&mut *tx)
            .await
            .map_err(internal)?;
    let owner_key = random_secret("ok_");
    sqlx::query(
        "INSERT INTO tenant_admins(tenant_id,email,role,api_key_hash) VALUES($1,$2,'owner',$3) ON CONFLICT(tenant_id,email) DO UPDATE SET api_key_hash=excluded.api_key_hash,active=true",
    )
    .bind(row.0)
    .bind(format!("terminal-{terminal_id}@terminal.local"))
    .bind(hash_key(&owner_key))
    .execute(&mut *tx)
    .await
    .map_err(internal)?;
    sqlx::query(
        "INSERT INTO terminals(id,tenant_id,unit_id,name,api_key_hash) VALUES($1,$2,$3,$4,$5)",
    )
    .bind(terminal_id)
    .bind(row.0)
    .bind(row.1)
    .bind(request.terminal_name.trim())
    .bind(hash_key(&terminal_key))
    .execute(&mut *tx)
    .await
    .map_err(internal)?;
    sqlx::query("UPDATE platform_activation_codes SET uses=uses+1 WHERE code_hash=$1")
        .bind(hash_key(&code))
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    let expires_at = trial_ends_at
        .filter(|value| *value > chrono::Utc::now())
        .or(period_ends_at)
        .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::days(30))
        .timestamp();
    let grace_until = expires_at + 7 * 86_400;
    sqlx::query("INSERT INTO licenses(tenant_id,unit_id,installation_id,status,expires_at,grace_until) VALUES($1,$2,$3,'active',$4,$5) ON CONFLICT(tenant_id,unit_id,installation_id) DO UPDATE SET status='active',expires_at=excluded.expires_at,grace_until=excluded.grace_until,updated_at=now()")
        .bind(row.0).bind(row.1).bind(request.installation_id.trim()).bind(expires_at).bind(grace_until).execute(&mut *tx).await.map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(Json(ActivationClaimed {
        tenant_id: row.0,
        unit_id: row.1,
        tenant_slug,
        tenant_name,
        terminal_id,
        terminal_key,
        owner_key,
        license: sign_license(LicenseClaims {
            tenant_id: row.0.to_string(),
            unit_id: row.1.to_string(),
            installation_id: request.installation_id.trim().into(),
            issued_at: chrono::Utc::now().timestamp(),
            expires_at,
            grace_until,
            nonce: Uuid::new_v4().to_string(),
        })?,
    }))
}

impl FromRequestParts<Arc<AppState>> for TerminalAuth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let tenant_id = header_uuid(&parts.headers, "x-tenant-id")?;
        let unit_id = header_uuid(&parts.headers, "x-unit-id")?;
        let terminal_id = header_uuid(&parts.headers, "x-terminal-id")?;
        let key = parts
            .headers
            .get("x-terminal-key")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "header x-terminal-key ausente"))?;
        let valid = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM terminals WHERE tenant_id=$1 AND unit_id=$2 AND id=$3 AND api_key_hash=$4 AND active=true)",
        )
        .bind(tenant_id)
        .bind(unit_id)
        .bind(terminal_id)
        .bind(hash_key(key))
        .fetch_one(&state.db)
        .await
        .map_err(internal)?;
        if !valid {
            return Err(error(StatusCode::UNAUTHORIZED, "terminal não autorizado"));
        }
        let _ = sqlx::query("UPDATE terminals SET last_seen_at=now() WHERE tenant_id=$1 AND id=$2")
            .bind(tenant_id)
            .bind(terminal_id)
            .execute(&state.db)
            .await;
        Ok(Self {
            tenant_id,
            unit_id,
            terminal_id,
        })
    }
}

impl FromRequestParts<Arc<AppState>> for OwnerAuth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let tenant_id = header_uuid(&parts.headers, "x-tenant-id")?;
        let key = parts
            .headers
            .get("x-owner-key")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "header x-owner-key ausente"))?;
        let valid = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM tenant_admins WHERE tenant_id=$1 AND api_key_hash=$2 AND active=true AND role IN ('owner','manager'))",
        ).bind(tenant_id).bind(hash_key(key)).fetch_one(&state.db).await.map_err(internal)?;
        if !valid {
            return Err(error(
                StatusCode::UNAUTHORIZED,
                "proprietário não autorizado",
            ));
        }
        // Mesma regra do painel web: assinatura vencida/revogada mantém a
        // leitura (GET/HEAD/OPTIONS) e bloqueia escritas com 402.
        if !matches!(
            parts.method,
            axum::http::Method::GET | axum::http::Method::HEAD | axum::http::Method::OPTIONS
        ) && !subscription_writable(&state.db, tenant_id).await?
        {
            return Err(error(
                StatusCode::PAYMENT_REQUIRED,
                "assinatura vencida: acesso somente leitura",
            ));
        }
        let _ = sqlx::query(
            "UPDATE tenant_admins SET last_seen_at=now() WHERE tenant_id=$1 AND api_key_hash=$2",
        )
        .bind(tenant_id)
        .bind(hash_key(key))
        .execute(&state.db)
        .await;
        Ok(Self { tenant_id })
    }
}

impl FromRequestParts<Arc<AppState>> for PanelAuth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        // Desktop: credencial de proprietário por header.
        if parts.headers.contains_key("x-owner-key") {
            let auth = OwnerAuth::from_request_parts(parts, state).await?;
            return Ok(Self {
                tenant_id: auth.tenant_id,
            });
        }
        // Web: sessão do usuário do tenant. O bloqueio de escrita por assinatura
        // vencida é aplicado dentro do TenantAuth.
        let auth = TenantAuth::from_request_parts(parts, state).await?;
        let privileged = matches!(auth.role.as_str(), "owner" | "admin" | "manager");
        if !privileged
            && !matches!(
                parts.method,
                axum::http::Method::GET | axum::http::Method::HEAD | axum::http::Method::OPTIONS
            )
        {
            return Err(error(
                StatusCode::FORBIDDEN,
                "perfil sem permissão para alterar o painel",
            ));
        }
        Ok(Self {
            tenant_id: auth.tenant_id,
        })
    }
}

fn internal(err: impl std::fmt::Display) -> ApiError {
    warn!(error = %err, "erro interno");
    error(StatusCode::INTERNAL_SERVER_ERROR, "erro interno")
}

async fn health(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, ApiError> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .map_err(internal)?;
    Ok(Json(
        serde_json::json!({ "status": "ok", "service": "commercectrl-backend" }),
    ))
}

async fn desktop_update(
    Path((_target, _arch, current_version)): Path<(String, String, String)>,
) -> impl IntoResponse {
    // Render recebe o manifesto assinado pelo pipeline; a URL do artefato pode
    // apontar para Supabase Storage. Sem manifesto ou na mesma versão, 204.
    let Ok(raw) = env::var("DESKTOP_UPDATE_MANIFEST_JSON") else {
        return StatusCode::NO_CONTENT.into_response();
    };
    let Ok(manifest) = serde_json::from_str::<Value>(&raw) else {
        return StatusCode::NO_CONTENT.into_response();
    };
    if manifest.get("version").and_then(Value::as_str) == Some(current_version.as_str()) {
        return StatusCode::NO_CONTENT.into_response();
    }
    ([(header::CONTENT_TYPE, "application/json")], raw).into_response()
}

async fn ingest_event(
    State(state): State<Arc<AppState>>,
    auth: TerminalAuth,
    Json(event): Json<CloudEventIn>,
) -> Result<impl IntoResponse, ApiError> {
    let payload = match event.payload {
        Value::String(raw) => serde_json::from_str(&raw)
            .map_err(|_| error(StatusCode::BAD_REQUEST, "payload JSON inválido"))?,
        value => value,
    };
    let mut tx = state.db.begin().await.map_err(internal)?;
    let inserted = sqlx::query(
        "INSERT INTO cloud_events (tenant_id,unit_id,terminal_id,event_uuid,entity,operation,payload,client_created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT (tenant_id,event_uuid) DO NOTHING",
    )
    .bind(auth.tenant_id).bind(auth.unit_id).bind(auth.terminal_id).bind(event.uuid)
    .bind(&event.entity).bind(&event.operation).bind(&payload).bind(event.created_at)
    .execute(&mut *tx).await.map_err(internal)?.rows_affected() == 1;

    if !inserted {
        tx.rollback().await.map_err(internal)?;
        return Ok(Json(CloudEventOut {
            accepted: true,
            duplicate: true,
        }));
    }

    match event.entity.as_str() {
        "sale" => materialize_sale(&mut tx, &auth, event.uuid, payload).await?,
        "stock_movement" => {
            materialize_stock(&mut tx, &auth, event.uuid, event.created_at, payload).await?
        }
        "product" => materialize_product(&mut tx, &auth, payload).await?,
        "user" => materialize_user(&mut tx, &auth, payload).await?,
        "employee" | "expense" | "supplier" | "customer" | "promotion" | "purchase_order"
        | "time_entry" | "cash_session" | "cash_movement" => {
            materialize_operational(&mut tx, &auth, &event.entity, payload).await?
        }
        _ => {}
    }
    tx.commit().await.map_err(internal)?;
    Ok(Json(CloudEventOut {
        accepted: true,
        duplicate: false,
    }))
}

async fn materialize_user(
    tx: &mut Transaction<'_, Postgres>,
    auth: &TerminalAuth,
    payload: Value,
) -> Result<(), ApiError> {
    let email = payload
        .get("email")
        .or_else(|| payload.get("username"))
        .and_then(Value::as_str)
        .map(|value| value.trim().to_lowercase())
        .filter(|value| value.contains('@') && value.len() <= 160)
        .ok_or_else(|| error(StatusCode::BAD_REQUEST, "usuário sem e-mail válido"))?;
    let password_hash = payload
        .get("password_hash")
        .and_then(Value::as_str)
        .filter(|value| value.starts_with("$argon2"))
        .ok_or_else(|| error(StatusCode::BAD_REQUEST, "usuário sem hash de senha"))?;
    let display_name = payload
        .get("display_name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&email);
    let role = match payload
        .get("role")
        .and_then(Value::as_str)
        .unwrap_or("cashier")
    {
        "owner" | "admin" => "owner",
        "manager" => "manager",
        "stock" => "stock",
        "finance" => "finance",
        "viewer" => "viewer",
        _ => "cashier",
    };
    let active = payload
        .get("active")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let must_change = payload
        .get("must_change_password")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let unit_id = if role == "cashier" || role == "viewer" {
        Some(auth.unit_id)
    } else {
        None
    };
    sqlx::query("INSERT INTO tenant_users(tenant_id,unit_id,email,password_hash,role,active,must_change_password,display_name,source,updated_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,'terminal',now()) ON CONFLICT(tenant_id,email) DO UPDATE SET password_hash=excluded.password_hash,unit_id=excluded.unit_id,role=excluded.role,active=excluded.active,must_change_password=excluded.must_change_password,display_name=excluded.display_name,source='terminal',updated_at=now()")
        .bind(auth.tenant_id)
        .bind(unit_id)
        .bind(&email)
        .bind(password_hash)
        .bind(role)
        .bind(active)
        .bind(must_change)
        .bind(display_name)
        .execute(&mut **tx)
        .await
        .map_err(internal)?;
    Ok(())
}

async fn materialize_product(
    tx: &mut Transaction<'_, Postgres>,
    auth: &TerminalAuth,
    payload: Value,
) -> Result<(), ApiError> {
    let item: ProductPayload = serde_json::from_value(payload)
        .map_err(|e| error(StatusCode::BAD_REQUEST, format!("produto inválido: {e}")))?;
    if item.ean.trim().is_empty()
        || item.description.trim().is_empty()
        || item.price_brl_cents < 0
        || item.min_stock < 0.0
    {
        return Err(error(StatusCode::BAD_REQUEST, "produto inválido"));
    }
    if item
        .image_url
        .as_ref()
        .is_some_and(|v| v.len() > 700000 || !v.starts_with("data:image/webp;base64,"))
    {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "imagem deve ser WebP e ter no máximo 500 KB",
        ));
    }
    sqlx::query("INSERT INTO products(tenant_id,ean,part_number,description,brand,price_brl_cents,min_stock,active,image_data_url) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT(tenant_id,ean) DO UPDATE SET part_number=excluded.part_number,description=excluded.description,brand=excluded.brand,price_brl_cents=excluded.price_brl_cents,min_stock=excluded.min_stock,active=excluded.active,image_data_url=COALESCE(excluded.image_data_url,products.image_data_url),version=products.version+1,updated_at=now()")
        .bind(auth.tenant_id).bind(item.ean).bind(item.part_number).bind(item.description).bind(item.brand).bind(item.price_brl_cents).bind(item.min_stock).bind(item.active).bind(item.image_url).execute(&mut **tx).await.map_err(internal)?;
    Ok(())
}

async fn materialize_operational(
    tx: &mut Transaction<'_, Postgres>,
    auth: &TerminalAuth,
    entity: &str,
    payload: Value,
) -> Result<(), ApiError> {
    let local_id = payload
        .get("local_id")
        .or_else(|| payload.get("id"))
        .and_then(Value::as_i64)
        .ok_or_else(|| error(StatusCode::BAD_REQUEST, "registro sem id local"))?;
    sqlx::query("INSERT INTO operational_records(tenant_id,unit_id,terminal_id,entity,local_id,payload) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(tenant_id,terminal_id,entity,local_id) DO UPDATE SET payload=operational_records.payload || excluded.payload,updated_at=now()")
        .bind(auth.tenant_id).bind(auth.unit_id).bind(auth.terminal_id).bind(entity).bind(local_id).bind(payload).execute(&mut **tx).await.map_err(internal)?;
    Ok(())
}

async fn materialize_sale(
    tx: &mut Transaction<'_, Postgres>,
    auth: &TerminalAuth,
    event_uuid: Uuid,
    payload: Value,
) -> Result<(), ApiError> {
    let sale: SalePayload = serde_json::from_value(payload)
        .map_err(|err| error(StatusCode::BAD_REQUEST, format!("venda inválida: {err}")))?;
    if sale.items.is_empty() || sale.total_brl_cents < 0 {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "venda sem itens ou total inválido",
        ));
    }
    let inserted = sqlx::query("INSERT INTO sales (tenant_id,unit_id,terminal_id,uuid,total_brl_cents,payment_method,client_created_at) VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT (tenant_id,uuid) DO NOTHING")
        .bind(auth.tenant_id).bind(auth.unit_id).bind(auth.terminal_id).bind(sale.uuid).bind(sale.total_brl_cents).bind(&sale.payment_method).bind(sale.created_at)
        .execute(&mut **tx).await.map_err(internal)?.rows_affected();
    if inserted == 0 {
        let existing = sqlx::query_as::<_, (Uuid, Uuid, i64, String, i64)>(
            "SELECT unit_id,terminal_id,total_brl_cents,payment_method,client_created_at FROM sales WHERE tenant_id=$1 AND uuid=$2",
        ).bind(auth.tenant_id).bind(sale.uuid).fetch_one(&mut **tx).await.map_err(internal)?;
        let items = sqlx::query_as::<_, (String, f64, i64)>(
            "SELECT ean,qty,price_brl_cents FROM sale_items WHERE tenant_id=$1 AND sale_uuid=$2 ORDER BY line_no",
        ).bind(auth.tenant_id).bind(sale.uuid).fetch_all(&mut **tx).await.map_err(internal)?;
        let same_items = items.len() == sale.items.len()
            && items.iter().zip(&sale.items).all(|(stored, incoming)| {
                stored.0 == incoming.ean
                    && stored.1 == incoming.qty
                    && stored.2 == incoming.price_brl_cents
            });
        if existing
            != (
                auth.unit_id,
                auth.terminal_id,
                sale.total_brl_cents,
                sale.payment_method.clone(),
                sale.created_at,
            )
            || !same_items
        {
            return Err(error(
                StatusCode::CONFLICT,
                "UUID de venda já usado com outros dados",
            ));
        }
        // A mesma venda pode chegar com outro UUID de transporte após recuperação da outbox.
        // Só a primeira inserção pode produzir movimentos de estoque.
        return Ok(());
    }
    for (line_no, item) in sale.items.into_iter().enumerate() {
        if !item.qty.is_finite() || item.qty <= 0.0 || item.price_brl_cents < 0 {
            return Err(error(StatusCode::BAD_REQUEST, "item inválido"));
        }
        sqlx::query("INSERT INTO products (tenant_id,ean,part_number,description,price_brl_cents) VALUES ($1,$2,$2,$2,$3) ON CONFLICT (tenant_id,ean) DO NOTHING")
            .bind(auth.tenant_id).bind(&item.ean).bind(item.price_brl_cents).execute(&mut **tx).await.map_err(internal)?;
        sqlx::query("INSERT INTO sale_items (tenant_id,sale_uuid,line_no,ean,qty,price_brl_cents) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING")
            .bind(auth.tenant_id).bind(sale.uuid).bind(line_no as i32).bind(&item.ean).bind(item.qty).bind(item.price_brl_cents).execute(&mut **tx).await.map_err(internal)?;
        sqlx::query("INSERT INTO stock_movements (tenant_id,unit_id,event_uuid,sale_uuid,ean,delta,reason,client_created_at) VALUES ($1,$2,$3,$4,$5,$6,'sale',$7) ON CONFLICT (tenant_id,event_uuid,ean) DO UPDATE SET delta=stock_movements.delta+excluded.delta")
            .bind(auth.tenant_id).bind(auth.unit_id).bind(event_uuid).bind(sale.uuid).bind(&item.ean).bind(-item.qty).bind(sale.created_at).execute(&mut **tx).await.map_err(internal)?;
    }
    Ok(())
}

async fn materialize_stock(
    tx: &mut Transaction<'_, Postgres>,
    auth: &TerminalAuth,
    event_uuid: Uuid,
    created_at: i64,
    payload: Value,
) -> Result<(), ApiError> {
    let movement: StockPayload = serde_json::from_value(payload).map_err(|err| {
        error(
            StatusCode::BAD_REQUEST,
            format!("movimento inválido: {err}"),
        )
    })?;
    sqlx::query("INSERT INTO stock_movements (tenant_id,unit_id,event_uuid,ean,delta,reason,client_created_at) VALUES ($1,$2,$3,$4,$5,'adjustment',$6) ON CONFLICT DO NOTHING")
        .bind(auth.tenant_id).bind(auth.unit_id).bind(event_uuid).bind(movement.ean).bind(movement.delta).bind(created_at).execute(&mut **tx).await.map_err(internal)?;
    Ok(())
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct SyncUserOut {
    id: Uuid,
    email: String,
    display_name: Option<String>,
    role: String,
    password_hash: String,
    must_change_password: bool,
    active: bool,
    updated_at: chrono::DateTime<chrono::Utc>,
}

/// Usuários que o terminal deve espelhar localmente para login offline.
async fn sync_users(
    State(state): State<Arc<AppState>>,
    auth: TerminalAuth,
    Query(query): Query<SyncUsersQuery>,
) -> Result<Json<Vec<SyncUserOut>>, ApiError> {
    let rows = sqlx::query_as::<_, SyncUserOut>(
        "SELECT id,email,display_name,role,password_hash,must_change_password,active,updated_at FROM tenant_users WHERE tenant_id=$1 AND (unit_id IS NULL OR unit_id=$2) AND ($3::bigint IS NULL OR updated_at > to_timestamp(($3::bigint)::float8 / 1000)) ORDER BY updated_at,email",
    )
    .bind(auth.tenant_id)
    .bind(auth.unit_id)
    .bind(query.since)
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;
    Ok(Json(rows))
}

async fn products(
    State(state): State<Arc<AppState>>,
    auth: TerminalAuth,
    Query(query): Query<ProductsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    // `updated_at` devolvido é o maior entre produto, ajustes por unidade e o
    // último movimento de estoque recebido — assim o cursor do PDV também
    // avança quando outro terminal vende/ajusta estoque nesta unidade.
    let sql = "
        SELECT p.ean,
               p.part_number,
               p.description,
               p.brand,
               COALESCE(s.price_brl_cents,p.price_brl_cents) AS price_brl_cents,
               COALESCE((SELECT SUM(m.delta) FROM stock_movements m
                         WHERE m.tenant_id=p.tenant_id AND m.unit_id=$2 AND m.ean=p.ean),0)::float8 AS stock_qty,
               COALESCE(s.min_stock,p.min_stock) AS min_stock,
               COALESCE(s.active,p.active) AS active,
               p.image_data_url AS image_url,
               GREATEST(p.updated_at,
                        COALESCE(s.updated_at,p.updated_at),
                        COALESCE((SELECT MAX(m2.received_at) FROM stock_movements m2
                                  WHERE m2.tenant_id=p.tenant_id AND m2.unit_id=$2 AND m2.ean=p.ean),p.updated_at)) AS updated_at
        FROM products p
        LEFT JOIN product_unit_settings s
               ON s.tenant_id=p.tenant_id AND s.unit_id=$2 AND s.ean=p.ean
        WHERE p.tenant_id=$1
          AND ($3::bigint IS NULL OR GREATEST(
                p.updated_at,
                COALESCE(s.updated_at,p.updated_at),
                COALESCE((SELECT MAX(m3.received_at) FROM stock_movements m3
                          WHERE m3.tenant_id=p.tenant_id AND m3.unit_id=$2 AND m3.ean=p.ean),p.updated_at)
              ) > to_timestamp(($3::bigint)::float8 / 1000))
        ORDER BY GREATEST(
                p.updated_at,
                COALESCE(s.updated_at,p.updated_at),
                COALESCE((SELECT MAX(m4.received_at) FROM stock_movements m4
                          WHERE m4.tenant_id=p.tenant_id AND m4.unit_id=$2 AND m4.ean=p.ean),p.updated_at)
              ), p.ean
        LIMIT $4";
    let rows = sqlx::query_as::<_, ProductOut>(sql)
        .bind(auth.tenant_id)
        .bind(auth.unit_id)
        .bind(query.since)
        .bind(query.limit.unwrap_or(1000).clamp(1, 5000))
        .fetch_all(&state.db)
        .await
        .map_err(internal)?;
    Ok(Json(rows))
}

async fn dashboard_summary(
    State(state): State<Arc<AppState>>,
    auth: PanelAuth,
) -> Result<Json<DashboardSummaryOut>, ApiError> {
    let today = chrono::Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp();
    let sales_today_brl_cents = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(total_brl_cents),0)::bigint FROM sales WHERE tenant_id=$1 AND client_created_at>=$2",
    )
    .bind(auth.tenant_id)
    .bind(today)
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;
    let items_today = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(si.qty),0)::bigint FROM sale_items si JOIN sales s ON s.tenant_id=si.tenant_id AND s.uuid=si.sale_uuid WHERE s.tenant_id=$1 AND s.client_created_at>=$2",
    )
    .bind(auth.tenant_id)
    .bind(today)
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;
    let new_customers_today = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)::bigint FROM operational_records WHERE tenant_id=$1 AND entity IN ('customer','customers') AND updated_at>=to_timestamp($2::float8)",
    )
    .bind(auth.tenant_id)
    .bind(today)
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;
    let low_stock_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)::bigint FROM (
            SELECT u.id,p.ean FROM units u
            JOIN products p ON p.tenant_id=u.tenant_id
            LEFT JOIN product_unit_settings ps ON ps.tenant_id=p.tenant_id AND ps.unit_id=u.id AND ps.ean=p.ean
            LEFT JOIN (SELECT tenant_id,unit_id,ean,SUM(delta) qty FROM stock_movements GROUP BY 1,2,3) sm
                ON sm.tenant_id=p.tenant_id AND sm.unit_id=u.id AND sm.ean=p.ean
            WHERE u.tenant_id=$1 AND COALESCE(ps.active,p.active)
              AND COALESCE(sm.qty,0) < COALESCE(ps.min_stock,p.min_stock)
        ) low_stock",
    )
    .bind(auth.tenant_id)
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;
    let daily = sqlx::query_as::<_, (i64, i64)>(
        "SELECT ((client_created_at-$2)/86400)::bigint AS day,COALESCE(SUM(total_brl_cents),0)::bigint FROM sales WHERE tenant_id=$1 AND client_created_at>=$2 GROUP BY 1",
    )
    .bind(auth.tenant_id)
    .bind(today)
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;
    let mut daily_sales = vec![0i64; 7];
    for (day, total) in daily {
        if (0..7).contains(&day) {
            daily_sales[day as usize] = total;
        }
    }
    // Índice 0 = hoje; o painel exibe do mais antigo para o mais recente.
    daily_sales.reverse();
    Ok(Json(DashboardSummaryOut {
        sales_today_brl_cents,
        items_today,
        new_customers_today,
        low_stock_count,
        daily_sales,
    }))
}

async fn tenant_sales_report(
    State(state): State<Arc<AppState>>,
    auth: PanelAuth,
    Query(query): Query<WebReportsQuery>,
) -> Result<Json<Vec<WebSalesRow>>, ApiError> {
    let from = query.from.unwrap_or(0);
    let to = query.to.unwrap_or(i64::MAX);
    if from > to {
        return Err(error(StatusCode::BAD_REQUEST, "período inválido"));
    }
    let payment = query
        .payment_method
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let terminal = query
        .terminal_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let product = query
        .product_query
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let rows = sqlx::query_as::<_, WebSalesRow>(
        "SELECT s.uuid::text uuid,s.total_brl_cents,s.payment_method,s.terminal_id::text terminal_id,
                (SELECT COUNT(*) FROM sale_items si WHERE si.tenant_id=s.tenant_id AND si.sale_uuid=s.uuid)::bigint items_count,
                s.client_created_at created_at
         FROM sales s
         WHERE s.tenant_id=$1 AND s.client_created_at>=$2 AND s.client_created_at<$3
           AND ($4::text IS NULL OR s.payment_method=$4)
           AND ($5::text IS NULL OR s.terminal_id::text=$5)
           AND ($6::text IS NULL OR EXISTS(SELECT 1 FROM sale_items si WHERE si.tenant_id=s.tenant_id AND si.sale_uuid=s.uuid AND si.ean ILIKE '%'||$6||'%'))
         ORDER BY s.client_created_at DESC LIMIT 5000",
    )
    .bind(auth.tenant_id)
    .bind(from)
    .bind(to)
    .bind(payment)
    .bind(terminal)
    .bind(product)
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;
    Ok(Json(rows))
}

async fn owner_overview(
    State(state): State<Arc<AppState>>,
    auth: PanelAuth,
    Query(query): Query<ReportsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let from = query.from.unwrap_or(0).max(0);
    let to = query.to.unwrap_or(i64::MAX);
    if from > to {
        return Err(error(StatusCode::BAD_REQUEST, "período inválido"));
    }
    let units = sqlx::query_as::<_, OwnerUnitOut>(
        "SELECT u.id unit_id,u.name unit_name,u.code unit_code,
         (SELECT COUNT(*) FROM sales s WHERE s.tenant_id=u.tenant_id AND s.unit_id=u.id AND s.client_created_at BETWEEN $2 AND $3)::bigint sales_count,
         COALESCE((SELECT SUM(s.total_brl_cents) FROM sales s WHERE s.tenant_id=u.tenant_id AND s.unit_id=u.id AND s.client_created_at BETWEEN $2 AND $3),0)::bigint total_brl_cents,
         (SELECT COALESCE(SUM(si.qty),0)::bigint FROM sale_items si JOIN sales s ON s.tenant_id=si.tenant_id AND s.uuid=si.sale_uuid WHERE s.tenant_id=u.tenant_id AND s.unit_id=u.id AND s.client_created_at BETWEEN $2 AND $3)::bigint items_count,
         (SELECT COUNT(DISTINCT ean) FROM stock_movements sm WHERE sm.tenant_id=u.tenant_id AND sm.unit_id=u.id)::bigint stock_skus,
         (SELECT COUNT(*) FROM products p LEFT JOIN product_unit_settings ps ON ps.tenant_id=p.tenant_id AND ps.unit_id=u.id AND ps.ean=p.ean LEFT JOIN (SELECT tenant_id,unit_id,ean,SUM(delta) qty FROM stock_movements GROUP BY tenant_id,unit_id,ean) sm ON sm.tenant_id=p.tenant_id AND sm.unit_id=u.id AND sm.ean=p.ean WHERE p.tenant_id=u.tenant_id AND COALESCE(ps.active,p.active) AND COALESCE(sm.qty,0) < COALESCE(ps.min_stock,p.min_stock))::bigint low_stock_skus
         FROM units u WHERE u.tenant_id=$1 ORDER BY u.name",
    ).bind(auth.tenant_id).bind(from).bind(to).fetch_all(&state.db).await.map_err(internal)?;
    let top_products = sqlx::query_as::<_, OwnerTopProductOut>(
        "SELECT si.ean,COALESCE(MAX(p.description),si.ean) description,SUM(si.qty)::float8 qty_sold,SUM(si.qty*si.price_brl_cents)::bigint revenue_brl_cents,COUNT(DISTINCT s.unit_id)::bigint units_sold_in FROM sale_items si JOIN sales s ON s.tenant_id=si.tenant_id AND s.uuid=si.sale_uuid LEFT JOIN products p ON p.tenant_id=si.tenant_id AND p.ean=si.ean WHERE s.tenant_id=$1 AND s.client_created_at BETWEEN $2 AND $3 GROUP BY si.ean ORDER BY revenue_brl_cents DESC,qty_sold DESC,si.ean LIMIT 10",
    ).bind(auth.tenant_id).bind(from).bind(to).fetch_all(&state.db).await.map_err(internal)?;
    let total_brl_cents = units.iter().map(|unit| unit.total_brl_cents).sum();
    let sales_count = units.iter().map(|unit| unit.sales_count).sum();
    Ok(Json(OwnerOverviewOut {
        total_brl_cents,
        sales_count,
        units,
        top_products,
    }))
}

async fn save_unit_product_setting(
    State(state): State<Arc<AppState>>,
    auth: OwnerAuth,
    Json(request): Json<UnitProductSettingRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if request.ean.trim().is_empty()
        || request.ean.len() > 100
        || request
            .price_brl_cents
            .as_ref()
            .is_some_and(|value| *value < 0)
        || request
            .min_stock
            .as_ref()
            .is_some_and(|value| !value.is_finite() || *value < 0.0)
    {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "configuração de produto inválida",
        ));
    }
    let affected = sqlx::query(
        "INSERT INTO product_unit_settings(tenant_id,unit_id,ean,price_brl_cents,min_stock,active)
         SELECT $1,$2,$3,$4,$5,$6 WHERE EXISTS(SELECT 1 FROM units WHERE tenant_id=$1 AND id=$2)
           AND EXISTS(SELECT 1 FROM products WHERE tenant_id=$1 AND ean=$3)
         ON CONFLICT(tenant_id,unit_id,ean) DO UPDATE SET price_brl_cents=excluded.price_brl_cents,min_stock=excluded.min_stock,active=excluded.active,updated_at=now()",
    ).bind(auth.tenant_id).bind(request.unit_id).bind(request.ean.trim()).bind(request.price_brl_cents).bind(request.min_stock).bind(request.active)
    .execute(&state.db).await.map_err(internal)?.rows_affected();
    if affected != 1 {
        return Err(error(
            StatusCode::NOT_FOUND,
            "unidade ou produto não encontrado",
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn save_owner_promotion(
    State(state): State<Arc<AppState>>,
    auth: OwnerAuth,
    Json(request): Json<OwnerPromotionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if request.title.trim().is_empty()
        || request.title.len() > 200
        || request.units.len() > 100
        || request
            .ends_at
            .as_ref()
            .is_some_and(|end| request.starts_at.as_ref().is_some_and(|start| end <= start))
        || request.units.iter().any(|unit| {
            unit.price_brl_cents
                .as_ref()
                .is_some_and(|price| *price < 0)
        })
    {
        return Err(error(StatusCode::BAD_REQUEST, "promoção inválida"));
    }
    let mut seen = std::collections::HashSet::new();
    if request.units.iter().any(|unit| !seen.insert(unit.unit_id)) {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "unidade repetida na promoção",
        ));
    }
    let id = request.id.unwrap_or_else(Uuid::new_v4);
    let scope = if request.units.is_empty() {
        "all_units"
    } else {
        "selected_units"
    };
    let mut tx = state.db.begin().await.map_err(internal)?;
    if let Some(ean) = request.ean.as_deref() {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM products WHERE tenant_id=$1 AND ean=$2)",
        )
        .bind(auth.tenant_id)
        .bind(ean)
        .fetch_one(&mut *tx)
        .await
        .map_err(internal)?;
        if !exists {
            return Err(error(
                StatusCode::NOT_FOUND,
                "produto da promoção não encontrado",
            ));
        }
    }
    let changed = sqlx::query("INSERT INTO promotions(id,tenant_id,ean,title,subtitle,price_label,starts_at,ends_at,active) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT(id) DO UPDATE SET ean=excluded.ean,title=excluded.title,subtitle=excluded.subtitle,price_label=excluded.price_label,starts_at=excluded.starts_at,ends_at=excluded.ends_at,active=excluded.active,updated_at=now() WHERE promotions.tenant_id=$2")
        .bind(id).bind(auth.tenant_id).bind(request.ean.as_deref()).bind(request.title.trim()).bind(request.subtitle.as_deref()).bind(request.price_label.as_deref()).bind(request.starts_at).bind(request.ends_at).bind(request.active)
        .execute(&mut *tx).await.map_err(internal)?.rows_affected();
    if changed != 1 {
        return Err(error(StatusCode::NOT_FOUND, "promoção não encontrada"));
    }
    sqlx::query("DELETE FROM promotion_units WHERE promotion_id=$1 AND tenant_id=$2")
        .bind(id)
        .bind(auth.tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    for unit in request.units {
        let inserted = sqlx::query("INSERT INTO promotion_units(promotion_id,tenant_id,unit_id,price_brl_cents,active) SELECT $1,$2,$3,$4,$5 WHERE EXISTS(SELECT 1 FROM units WHERE tenant_id=$2 AND id=$3)")
            .bind(id).bind(auth.tenant_id).bind(unit.unit_id).bind(unit.price_brl_cents).bind(unit.active).execute(&mut *tx).await.map_err(internal)?.rows_affected();
        if inserted != 1 {
            return Err(error(
                StatusCode::NOT_FOUND,
                "unidade da promoção não encontrada",
            ));
        }
    }
    tx.commit().await.map_err(internal)?;
    Ok(Json(serde_json::json!({"id": id, "scope": scope})))
}

async fn create_display_channel(
    State(state): State<Arc<AppState>>,
    auth: OwnerAuth,
    Json(request): Json<DisplayChannelRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if request.name.trim().is_empty() || request.name.len() > 100 {
        return Err(error(StatusCode::BAD_REQUEST, "nome do canal inválido"));
    }
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM units WHERE tenant_id=$1 AND id=$2)",
    )
    .bind(auth.tenant_id)
    .bind(request.unit_id)
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;
    if !exists {
        return Err(error(StatusCode::NOT_FOUND, "unidade não encontrada"));
    }
    let id = Uuid::new_v4();
    let token = format!("tv_{}", Uuid::new_v4().simple());
    sqlx::query(
        "INSERT INTO display_channels(id,tenant_id,unit_id,name,token_hash) VALUES($1,$2,$3,$4,$5)",
    )
    .bind(id)
    .bind(auth.tenant_id)
    .bind(request.unit_id)
    .bind(request.name.trim())
    .bind(hash_key(&token))
    .execute(&state.db)
    .await
    .map_err(internal)?;
    Ok((
        StatusCode::CREATED,
        Json(DisplayChannelCreated {
            id,
            path: format!("/api/v1/display/{token}/promotions"),
            token,
        }),
    ))
}

async fn display_promotions(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let channel: Option<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT tenant_id,unit_id FROM display_channels WHERE token_hash=$1 AND active=true",
    )
    .bind(hash_key(&token))
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?;
    let Some((tenant_id, unit_id)) = channel else {
        return Err(error(StatusCode::NOT_FOUND, "canal de TV não encontrado"));
    };
    let rows = sqlx::query_as::<_, DisplayPromotionOut>(
        "SELECT p.title,p.subtitle,COALESCE(CASE WHEN pu.price_brl_cents IS NULL THEN NULL ELSE 'R$ ' || (pu.price_brl_cents / 100)::text || ',' || lpad((pu.price_brl_cents % 100)::text,2,'0') END,p.price_label,'') price_label,product.image_data_url image_url FROM promotions p LEFT JOIN promotion_units pu ON pu.promotion_id=p.id AND pu.tenant_id=$1 AND pu.unit_id=$2 LEFT JOIN products product ON product.tenant_id=p.tenant_id AND product.ean=p.ean WHERE p.tenant_id=$1 AND p.active=true AND (p.starts_at IS NULL OR p.starts_at<=now()) AND (p.ends_at IS NULL OR p.ends_at>now()) AND (NOT EXISTS(SELECT 1 FROM promotion_units scoped WHERE scoped.promotion_id=p.id) OR COALESCE(pu.active,false)) ORDER BY p.updated_at DESC,p.id",
    ).bind(tenant_id).bind(unit_id).fetch_all(&state.db).await.map_err(internal)?;
    Ok(Json(rows))
}

async fn online_tv_page(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Result<axum::response::Html<&'static str>, ApiError> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM display_channels WHERE token_hash=$1 AND active=true)",
    )
    .bind(hash_key(&token))
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;
    if !exists {
        return Err(error(StatusCode::NOT_FOUND, "canal de TV não encontrado"));
    }
    Ok(axum::response::Html(ONLINE_TV_CATALOG_HTML))
}

async fn owner_branding(
    State(state): State<Arc<AppState>>,
    auth: PanelAuth,
) -> Result<Json<TenantBranding>, ApiError> {
    let branding = sqlx::query_as::<_, TenantBranding>(
        "SELECT display_name,logo_data_url,primary_color FROM tenant_branding WHERE tenant_id=$1",
    )
    .bind(auth.tenant_id)
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?;
    let branding = match branding {
        Some(value) => value,
        None => {
            let name = sqlx::query_scalar::<_, String>("SELECT name FROM tenants WHERE id=$1")
                .bind(auth.tenant_id)
                .fetch_optional(&state.db)
                .await
                .map_err(internal)?
                .unwrap_or_else(|| "CommerceCTRL".into());
            TenantBranding {
                display_name: name,
                logo_data_url: None,
                primary_color: Some("#0D5C3A".into()),
            }
        }
    };
    Ok(Json(branding))
}

async fn save_owner_branding(
    State(state): State<Arc<AppState>>,
    auth: PanelAuth,
    Json(input): Json<TenantBrandingInput>,
) -> Result<Json<TenantBranding>, ApiError> {
    let name = input.display_name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(error(StatusCode::BAD_REQUEST, "nome do mercado inválido"));
    }
    if input
        .logo_data_url
        .as_ref()
        .is_some_and(|logo| logo.len() > 2_800_000 || !logo.starts_with("data:image/"))
    {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "logo inválida ou maior que 2 MB",
        ));
    }
    if let Some(cor) = &input.primary_color {
        if !cor.is_empty() && !cor.starts_with('#') {
            return Err(error(StatusCode::BAD_REQUEST, "cor deve começar com #"));
        }
        if cor.len() != 7 {
            return Err(error(
                StatusCode::BAD_REQUEST,
                "cor deve estar no formato #RRGGBB",
            ));
        }
    }
    let branding = sqlx::query_as::<_, TenantBranding>("INSERT INTO tenant_branding(tenant_id,display_name,logo_data_url,primary_color) VALUES($1,$2,$3,$4) ON CONFLICT(tenant_id) DO UPDATE SET display_name=excluded.display_name,logo_data_url=excluded.logo_data_url,primary_color=excluded.primary_color,updated_at=now() RETURNING display_name,logo_data_url,primary_color")
        .bind(auth.tenant_id).bind(name).bind(input.logo_data_url).bind(input.primary_color).fetch_one(&state.db).await.map_err(internal)?;
    Ok(Json(branding))
}

async fn demo_products_sprite() -> impl IntoResponse {
    (
        [
            ("content-type", "image/png"),
            ("cache-control", "public, max-age=31536000, immutable"),
        ],
        include_bytes!("../assets/demo-products-v1.png").as_slice(),
    )
}

async fn demo_product_image(Path(ean): Path<String>) -> impl IntoResponse {
    let cell = match ean.as_str() {
        "78900001" => (0, 0),
        "78900002" => (1, 0),
        "78900003" => (2, 0),
        "78900004" => (3, 0),
        "78900005" => (0, 1),
        "78900006" => (1, 1),
        "78900007" => (2, 1),
        "78900008" => (3, 1),
        _ => (1, 1),
    };
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 448 448"><rect width="448" height="448" fill="white"/><image href="/assets/demo-products-v1.png" width="1792" height="896" x="{}" y="{}" preserveAspectRatio="none"/></svg>"#,
        -448 * cell.0,
        -448 * cell.1
    );
    (
        [
            ("content-type", "image/svg+xml; charset=utf-8"),
            ("cache-control", "public, max-age=86400"),
        ],
        svg,
    )
}

async fn sales_by_unit(
    State(state): State<Arc<AppState>>,
    auth: TerminalAuth,
    Query(query): Query<ReportsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let from = query.from.unwrap_or(0).max(0);
    let to = query.to.unwrap_or(i64::MAX);
    if from > to {
        return Err(error(StatusCode::BAD_REQUEST, "período inválido"));
    }
    let rows = sqlx::query_as::<_, UnitSalesOut>(
        "SELECT u.id AS unit_id,u.name AS unit_name,(SELECT COUNT(*) FROM sales s WHERE s.tenant_id=u.tenant_id AND s.unit_id=u.id AND s.client_created_at BETWEEN $2 AND $3)::bigint AS sales_count,COALESCE((SELECT SUM(s.total_brl_cents) FROM sales s WHERE s.tenant_id=u.tenant_id AND s.unit_id=u.id AND s.client_created_at BETWEEN $2 AND $3),0)::bigint AS total_brl_cents,(SELECT COUNT(*) FROM sale_items si JOIN sales s ON s.tenant_id=si.tenant_id AND s.uuid=si.sale_uuid WHERE s.tenant_id=u.tenant_id AND s.unit_id=u.id AND s.client_created_at BETWEEN $2 AND $3)::bigint AS items_count FROM units u WHERE u.tenant_id=$1 ORDER BY u.name",
    )
    .bind(auth.tenant_id)
    .bind(from)
    .bind(to)
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;
    Ok(Json(rows))
}

/// Divide um script SQL em comandos individuais para que uma falha não
/// desfaça os comandos anteriores (o protocolo simples do Postgres executa
/// múltiplos statements em uma transação implícita).
fn sql_statements(script: &str) -> Vec<&str> {
    script
        .split(';')
        .map(str::trim)
        .filter(|statement| {
            !statement.is_empty()
                && statement.lines().any(|line| {
                    let line = line.trim();
                    !line.is_empty() && !line.starts_with("--")
                })
        })
        .collect()
}

/// Compatibilidade de schema: enquanto as migrations rodam manualmente no
/// Supabase, aplica as pendentes (0015–0018) na inicialização. Cada comando é
/// idempotente e executado isoladamente; falhas são registradas sem impedir o
/// boot quando o banco já está parcialmente migrado.
async fn ensure_identity_schema(pool: &PgPool) {
    let ready = sqlx::query_scalar::<_, bool>(
        "SELECT
            (SELECT count(*) FROM information_schema.columns WHERE table_schema='public' AND table_name='tenants' AND column_name='slug') > 0
            AND (SELECT count(*) FROM information_schema.columns WHERE table_schema='public' AND table_name='tenant_subscriptions' AND column_name='billing_mode') > 0
            AND (SELECT count(*) FROM information_schema.columns WHERE table_schema='public' AND table_name='tenant_subscriptions' AND column_name='courtesy_until') > 0
            AND (SELECT count(*) FROM information_schema.columns WHERE table_schema='public' AND table_name='tenant_subscriptions' AND column_name='courtesy_note') > 0
            AND (SELECT count(*) FROM information_schema.columns WHERE table_schema='public' AND table_name='tenant_users' AND column_name='display_name') > 0
            AND (SELECT count(*) FROM information_schema.columns WHERE table_schema='public' AND table_name='tenant_users' AND column_name='source') > 0
            AND (SELECT count(*) FROM information_schema.columns WHERE table_schema='public' AND table_name='tenant_users' AND column_name='revoked_at') > 0
            AND (SELECT count(*) FROM information_schema.columns WHERE table_schema='public' AND table_name='tenant_users' AND column_name='updated_at') > 0",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if ready {
        return;
    }
    warn!("schema de identidade/cortesia ausente; aplicando migrations pendentes na inicialização");
    for (version, script) in [
        (
            "0015_tenant_users",
            include_str!("../migrations/0015_tenant_users.sql"),
        ),
        (
            "0016_login_rate_limit",
            include_str!("../migrations/0016_login_rate_limit.sql"),
        ),
        (
            "0017_tenant_branding_color",
            include_str!("../migrations/0017_tenant_branding_color.sql"),
        ),
        (
            "0018_courtesy_identity",
            include_str!("../migrations/0018_courtesy_identity.sql"),
        ),
    ] {
        for statement in sql_statements(script) {
            if let Err(error) = sqlx::raw_sql(statement).execute(pool).await {
                warn!(
                    "migration {version} falhou em [{}]: {error}",
                    statement.chars().take(90).collect::<String>()
                );
            }
        }
    }
}

/// Aplica a 0019 quando o banco ainda não tem as colunas de arquivamento.
async fn ensure_platform_ops_schema(pool: &PgPool) {
    let ready = sqlx::query_scalar::<_, bool>(
        "SELECT
            (SELECT count(*) FROM information_schema.columns WHERE table_schema='public' AND table_name='tenants' AND column_name='archived_at') > 0
            AND (SELECT count(*) FROM information_schema.columns WHERE table_schema='public' AND table_name='units' AND column_name='active') > 0
            AND to_regclass('public.tenant_plan_history') IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if ready {
        return;
    }
    warn!("schema de operação da plataforma ausente; aplicando 0019 na inicialização");
    for statement in sql_statements(include_str!("../migrations/0019_platform_ops.sql")) {
        if let Err(error) = sqlx::raw_sql(statement).execute(pool).await {
            warn!(
                "migration 0019 falhou em [{}]: {error}",
                statement.chars().take(90).collect::<String>()
            );
        }
    }
}

async fn system_audit(pool: &PgPool, action: &str, tenant_id: Uuid, metadata: Value) {
    if let Err(cause) = sqlx::query(
        "INSERT INTO platform_audit_events(admin_id,action,tenant_id,metadata) VALUES(NULL,$1,$2,$3)",
    )
    .bind(action)
    .bind(tenant_id)
    .bind(metadata)
    .execute(pool)
    .await
    {
        warn!(?cause, "falha ao gravar auditoria automática");
    }
}

/// Rede de segurança da cobrança: sem webhook, trial/assinatura vencida entra
/// em `past_due` e, depois de 7 dias de carência, em `suspended`.
async fn billing_sweep(pool: &PgPool) {
    let expired_trials = sqlx::query_scalar::<_, Uuid>(
        "UPDATE tenant_subscriptions s SET status='past_due',updated_at=now()
         FROM tenants t
         WHERE t.id=s.tenant_id AND t.archived_at IS NULL
           AND s.status='trial' AND s.trial_ends_at IS NOT NULL AND s.trial_ends_at<=now()
         RETURNING s.tenant_id",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    for tenant_id in expired_trials {
        system_audit(pool, "subscription.trial_expired", tenant_id, serde_json::json!({})).await;
    }

    let overdue = sqlx::query_scalar::<_, Uuid>(
        "UPDATE tenant_subscriptions s SET status='past_due',updated_at=now()
         FROM tenants t
         WHERE t.id=s.tenant_id AND t.archived_at IS NULL
           AND s.status='active' AND s.billing_mode='paid'
           AND s.current_period_ends_at IS NOT NULL AND s.current_period_ends_at<=now()
         RETURNING s.tenant_id",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    for tenant_id in overdue {
        system_audit(
            pool,
            "subscription.period_ended",
            tenant_id,
            serde_json::json!({}),
        )
        .await;
    }

    let courtesy_over = sqlx::query_scalar::<_, Uuid>(
        "UPDATE tenant_subscriptions s SET status='past_due',updated_at=now()
         FROM tenants t
         WHERE t.id=s.tenant_id AND t.archived_at IS NULL
           AND s.status='active' AND s.billing_mode='courtesy'
           AND s.courtesy_until IS NOT NULL AND s.courtesy_until<=now()
         RETURNING s.tenant_id",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    for tenant_id in courtesy_over {
        system_audit(
            pool,
            "subscription.courtesy_ended",
            tenant_id,
            serde_json::json!({}),
        )
        .await;
    }

    let suspended = sqlx::query_scalar::<_, Uuid>(
        "UPDATE tenant_subscriptions s SET status='suspended',updated_at=now()
         FROM tenants t
         WHERE t.id=s.tenant_id AND t.archived_at IS NULL
           AND s.status='past_due'
           AND COALESCE(s.courtesy_until,s.current_period_ends_at,s.trial_ends_at)<=now()-interval '7 days'
         RETURNING s.tenant_id",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    for tenant_id in &suspended {
        system_audit(
            pool,
            "subscription.suspended_for_billing",
            *tenant_id,
            serde_json::json!({"grace_days": 7}),
        )
        .await;
    }
    if !suspended.is_empty() {
        let _ = sqlx::query(
            "UPDATE licenses l SET status='suspended',updated_at=now()
             FROM tenant_subscriptions s
             WHERE s.tenant_id=l.tenant_id AND s.status='suspended' AND l.status='active'",
        )
        .execute(pool)
        .await;
        info!(count = suspended.len(), "assinaturas suspensas por falta de pagamento");
    }
}

async fn run_billing_sweeper(pool: PgPool) {
    let hours = env::var("BILLING_SWEEP_HOURS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(6)
        .clamp(1, 24);
    let mut ticker =
        tokio::time::interval(std::time::Duration::from_secs(hours * 60 * 60));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        billing_sweep(&pool).await;
    }
}

async fn bootstrap(pool: &PgPool) -> Result<(), sqlx::Error> {
    ensure_identity_schema(pool).await;
    ensure_platform_ops_schema(pool).await;
    if let (Ok(email), Ok(password)) = (
        env::var("SUPERADMIN_EMAIL"),
        env::var("SUPERADMIN_PASSWORD"),
    ) {
        if !email.trim().is_empty() && password.len() >= 12 {
            let password_hash = password_hash(&password)
                .map_err(|_| sqlx::Error::Protocol("hash da conta master inválido".into()))?;
            sqlx::query("INSERT INTO platform_admins(id,email,password_hash,must_change_password) VALUES($1,$2,$3,true) ON CONFLICT(email) DO NOTHING")
                .bind(Uuid::new_v4())
                .bind(email.trim().to_lowercase())
                .bind(password_hash)
                .execute(pool)
                .await?;
        } else {
            warn!("SUPERADMIN_PASSWORD ignorada: use pelo menos 12 caracteres");
        }
    }
    let tenant_id = env_uuid(
        "BOOTSTRAP_TENANT_ID",
        "00000000-0000-0000-0000-000000000001",
    );
    let unit_id = env_uuid("BOOTSTRAP_UNIT_ID", "00000000-0000-0000-0000-000000000101");
    let terminal_id = env_uuid(
        "BOOTSTRAP_TERMINAL_ID",
        "00000000-0000-0000-0000-000000001001",
    );
    let key = env::var("BOOTSTRAP_TERMINAL_KEY").unwrap_or_else(|_| "commercectrl-dev-key".into());
    let owner_key =
        env::var("BOOTSTRAP_OWNER_KEY").unwrap_or_else(|_| "commercectrl-owner-dev-key".into());
    let has_slug = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM information_schema.columns WHERE table_schema='public' AND table_name='tenants' AND column_name='slug')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    let mut tx = pool.begin().await?;
    let tenant_name =
        env::var("BOOTSTRAP_TENANT_NAME").unwrap_or_else(|_| "Mercadinho Martins".into());
    if has_slug {
        sqlx::query("INSERT INTO tenants(id,name,slug) VALUES($1,$2,$3) ON CONFLICT(id) DO UPDATE SET name=excluded.name,slug=COALESCE(tenants.slug,excluded.slug)")
            .bind(tenant_id)
            .bind(&tenant_name)
            .bind(slugify(&tenant_name))
            .execute(&mut *tx)
            .await?;
    } else {
        warn!("coluna tenants.slug ausente; bootstrap seguirá sem slug até a 0018 ser aplicada");
        sqlx::query("INSERT INTO tenants(id,name) VALUES($1,$2) ON CONFLICT(id) DO UPDATE SET name=excluded.name")
            .bind(tenant_id)
            .bind(&tenant_name)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("INSERT INTO tenant_subscriptions(tenant_id,plan,status,trial_ends_at,current_period_ends_at) VALUES($1,'profissional','trial',now()+interval '3 months',now()+interval '3 months') ON CONFLICT(tenant_id) DO NOTHING")
        .bind(tenant_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO tenant_branding(tenant_id,display_name) VALUES($1,$2) ON CONFLICT(tenant_id) DO UPDATE SET display_name=excluded.display_name")
        .bind(tenant_id).bind(env::var("BOOTSTRAP_TENANT_NAME").unwrap_or_else(|_| "Mercadinho Martins".into())).execute(&mut *tx).await?;
    // A instalação local demonstra um único cliente com quatro lojas. IDs estáveis tornam
    // o bootstrap idempotente e permitem que cada caixa receba a unidade correta por ambiente.
    let units = [
        (
            unit_id,
            env::var("BOOTSTRAP_UNIT_NAME").unwrap_or_else(|_| "Matriz Centro".into()),
            "MATRIZ",
        ),
        (
            Uuid::parse_str("00000000-0000-0000-0000-000000000102").unwrap(),
            "Vila Industrial".into(),
            "VILA",
        ),
        (
            Uuid::parse_str("00000000-0000-0000-0000-000000000103").unwrap(),
            "Mogi Moderno".into(),
            "MOGI",
        ),
        (
            Uuid::parse_str("00000000-0000-0000-0000-000000000104").unwrap(),
            "Jardim Universo".into(),
            "UNIVERSO",
        ),
    ];
    for (id, name, code) in &units {
        sqlx::query("INSERT INTO units(id,tenant_id,name,code) VALUES($1,$2,$3,$4) ON CONFLICT(id) DO UPDATE SET name=excluded.name,code=excluded.code")
            .bind(id).bind(tenant_id).bind(name).bind(code).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO terminals(id,tenant_id,unit_id,name,api_key_hash) VALUES($1,$2,$3,$4,$5) ON CONFLICT(id) DO UPDATE SET api_key_hash=excluded.api_key_hash,active=true").bind(terminal_id).bind(tenant_id).bind(unit_id).bind("Caixa 01").bind(hash_key(&key)).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO tenant_admins(tenant_id,email,role,api_key_hash) VALUES($1,$2,'owner',$3) ON CONFLICT(tenant_id,email) DO UPDATE SET api_key_hash=excluded.api_key_hash,active=true")
        .bind(tenant_id).bind(env::var("BOOTSTRAP_OWNER_EMAIL").unwrap_or_else(|_| "owner@local.invalid".into())).bind(hash_key(&owner_key)).execute(&mut *tx).await?;
    let seed_products = [
        (
            "78900001",
            "PROD001",
            "Vinho Tinto Suave",
            "Adega",
            3000_i64,
            8.0_f64,
        ),
        (
            "78900002",
            "PROD002",
            "Cerveja Artesanal IPA",
            "Commerce",
            1500,
            40.0,
        ),
        (
            "78900003",
            "PROD003",
            "Whisky 12 Anos",
            "Reserva",
            12000,
            15.0,
        ),
        (
            "78900004",
            "PROD004",
            "Gin Importado",
            "London",
            13000,
            12.0,
        ),
        ("78900005", "PROD005", "Água Tônica", "Fresh", 500, 3.0),
        ("78900006", "PROD006", "Energético", "Power", 800, 50.0),
        ("78900007", "PROD007", "Saca-rolhas", "Casa", 2500, 2.0),
        (
            "78900008",
            "PROD008",
            "Cerveja Pilsen Pack 6",
            "Commerce",
            2200,
            11.0,
        ),
    ];
    for (ean, part_number, description, brand, price, initial_stock) in seed_products {
        sqlx::query("INSERT INTO products(tenant_id,ean,part_number,description,brand,price_brl_cents) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(tenant_id,ean) DO UPDATE SET part_number=excluded.part_number,description=excluded.description,brand=excluded.brand,price_brl_cents=excluded.price_brl_cents,version=products.version+1,updated_at=now()")
            .bind(tenant_id).bind(ean).bind(part_number).bind(description).bind(brand).bind(price).execute(&mut *tx).await?;
        for (position, (seed_unit_id, _, _)) in units.iter().enumerate() {
            // Estoque e preço mínimo variam por filial para que a visão consolidada seja útil
            // já na demonstração, sem misturar o estoque de uma loja com outra.
            let stock = initial_stock * [1.0, 0.72, 1.18, 0.55][position];
            sqlx::query("INSERT INTO stock_movements(tenant_id,unit_id,event_uuid,ean,delta,reason,client_created_at) SELECT $1,$2,gen_random_uuid(),$3,$4,'seed',extract(epoch from now())::bigint WHERE NOT EXISTS(SELECT 1 FROM stock_movements WHERE tenant_id=$1 AND unit_id=$2 AND ean=$3 AND reason='seed')")
                .bind(tenant_id).bind(seed_unit_id).bind(ean).bind(stock).execute(&mut *tx).await?;
            let local_price = price + [0_i64, 25, -15, 40][position];
            sqlx::query("INSERT INTO product_unit_settings(tenant_id,unit_id,ean,price_brl_cents,min_stock,active) VALUES($1,$2,$3,$4,$5,true) ON CONFLICT(tenant_id,unit_id,ean) DO NOTHING")
                .bind(tenant_id).bind(seed_unit_id).bind(ean).bind(local_price).bind((initial_stock * 0.2).max(2.0)).execute(&mut *tx).await?;
        }
    }
    let _ = tx.commit().await;
    // Usuários de demonstração para login web do tenant (dono e caixa)
    let dono_hash = password_hash("Comercio2024!")
        .map_err(|_| sqlx::Error::Protocol("hash inválido".into()))?;
    let caixa_hash =
        password_hash("Caixa2024!").map_err(|_| sqlx::Error::Protocol("hash inválido".into()))?;
    sqlx::query("INSERT INTO tenant_users(tenant_id,email,password_hash,role,active,must_change_password) VALUES($1,$2,$3,'owner',true,false) ON CONFLICT(tenant_id,email) DO UPDATE SET password_hash=excluded.password_hash,role=excluded.role,active=true")
        .bind(tenant_id).bind("dono@mercadinho.com.br").bind(dono_hash).execute(pool).await?;
    sqlx::query("INSERT INTO tenant_users(tenant_id,unit_id,email,password_hash,role,active,must_change_password) VALUES($1,$2,$3,$4,'cashier',true,false) ON CONFLICT(tenant_id,email) DO UPDATE SET password_hash=excluded.password_hash,unit_id=excluded.unit_id,role=excluded.role,active=true")
        .bind(tenant_id).bind(unit_id).bind("caixa@mercadinho.com.br").bind(caixa_hash).execute(pool).await?;
    Ok(())
}

fn env_uuid(name: &str, default: &str) -> Uuid {
    Uuid::parse_str(&env::var(name).unwrap_or_else(|_| default.into()))
        .expect("UUID de bootstrap inválido")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL ausente");
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await?;
    // Migrations aplicadas manualmente via psql (0001..0017 já estão no banco e registradas em _sqlx_migrations).
    // Não executar sqlx::migrate!() automaticamente para evitar VersionMismatch de checksums.
    bootstrap(&pool).await?;
    if env::var("BILLING_SWEEP_ENABLED")
        .map(|value| value != "false" && value != "0")
        .unwrap_or(true)
    {
        tokio::spawn(run_billing_sweeper(pool.clone()));
    }
    let state = Arc::new(AppState { db: pool });
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/public/license-key", get(license_public_key))
        .route("/api/v1/platform/auth/login", post(platform_login))
        .route("/api/v1/platform/auth/logout", post(platform_logout))
        .route("/api/v1/platform/me", get(platform_me))
        .route("/api/v1/auth/login", post(tenant_login))
        .route("/api/v1/auth/logout", post(tenant_logout))
        .route("/api/v1/auth/me", get(tenant_me))
        .route("/api/v1/auth/password", post(tenant_change_password))
        .route("/api/v1/commercial/leads", post(commercial_lead))
        .route(
            "/api/v1/platform/commercial/catalog",
            get(platform_commercial_catalog),
        )
        .route(
            "/api/v1/platform/commercial/equipment",
            post(platform_create_equipment),
        )
        .route("/api/v1/platform/tenants", get(platform_tenants))
        .route("/api/v1/platform/tenants", post(platform_create_tenant))
        .route(
            "/api/v1/platform/tenants/{tenant_id}/asaas-customer",
            post(platform_create_asaas_customer),
        )
        .route(
            "/api/v1/platform/tenants/{tenant_id}/subscription-payment",
            post(platform_create_subscription_payment),
        )
        .route(
            "/api/v1/platform/tenants/{tenant_id}/payment-status",
            get(platform_payment_status),
        )
        .route(
            "/api/v1/platform/tenants/{tenant_id}",
            get(platform_tenant_detail)
                .patch(platform_update_tenant)
                .delete(platform_archive_tenant),
        )
        .route(
            "/api/v1/platform/tenants/{tenant_id}/unarchive",
            post(platform_unarchive_tenant),
        )
        .route(
            "/api/v1/platform/tenants/{tenant_id}/plan",
            post(platform_change_plan),
        )
        .route(
            "/api/v1/platform/tenants/{tenant_id}/plan-history",
            get(platform_plan_history),
        )
        .route(
            "/api/v1/platform/tenants/{tenant_id}/owner-password",
            post(platform_reset_owner_password),
        )
        .route(
            "/api/v1/platform/tenants/{tenant_id}/units",
            get(platform_tenant_units).post(platform_create_unit),
        )
        .route(
            "/api/v1/platform/tenants/{tenant_id}/units/{unit_id}",
            patch(platform_update_unit),
        )
        .route(
            "/api/v1/platform/tenants/{tenant_id}/terminals",
            get(platform_tenant_terminals),
        )
        .route(
            "/api/v1/platform/tenants/{tenant_id}/terminals/{terminal_id}",
            patch(platform_update_terminal),
        )
        .route(
            "/api/v1/platform/tenants/{tenant_id}/users",
            get(platform_tenant_users),
        )
        .route(
            "/api/v1/platform/tenants/{tenant_id}/users/{user_id}",
            patch(platform_update_user),
        )
        .route(
            "/api/v1/platform/tenants/{tenant_id}/users/{user_id}/password",
            post(platform_reset_user_password),
        )
        .route(
            "/api/v1/platform/tenants/{tenant_id}/payments",
            get(platform_tenant_payments),
        )
        .route(
            "/api/v1/platform/tenants/{tenant_id}/flags",
            get(platform_tenant_flags),
        )
        .route(
            "/api/v1/platform/tenants/{tenant_id}/flags/{key}",
            put(platform_set_tenant_flag),
        )
        .route(
            "/api/v1/platform/tenants/{tenant_id}/impersonate",
            post(platform_impersonate_tenant),
        )
        .route("/api/v1/platform/billing/queue", get(platform_billing_queue))
        .route("/api/v1/platform/audit", get(platform_audit_list))
        .route("/api/v1/platform/metrics", get(platform_metrics))
        .route("/api/v1/platform/analytics", get(platform_analytics))
        .route("/api/v1/platform/leads", get(platform_leads))
        .route(
            "/api/v1/platform/leads/{lead_id}",
            patch(platform_update_lead),
        )
        .route("/api/v1/platform/flags", get(platform_flags))
        .route(
            "/api/v1/platform/flags/{key}",
            patch(platform_update_flag),
        )
        .route(
            "/api/v1/platform/export/tenants.csv",
            get(platform_export_tenants),
        )
        .route(
            "/api/v1/platform/export/payments.csv",
            get(platform_export_payments),
        )
        .route(
            "/api/v1/platform/export/audit.csv",
            get(platform_export_audit),
        )
        .route(
            "/api/v1/platform/tenants/{tenant_id}/units/{unit_id}/activation-codes",
            post(platform_create_activation_code),
        )
        .route(
            "/api/v1/platform/tenants/{tenant_id}/suspend",
            post(platform_suspend_tenant),
        )
        .route(
            "/api/v1/platform/tenants/{tenant_id}/restore",
            post(platform_restore_tenant),
        )
        .route(
            "/api/v1/platform/tenants/{tenant_id}/courtesy",
            post(platform_set_courtesy),
        )
        .route(
            "/api/v1/platform/activation-codes/{code_id}/revoke",
            post(platform_revoke_activation_code),
        )
        .route("/api/v1/activation/claim", post(claim_activation_code))
        .route("/api/v1/signup", post(public_signup))
        .route("/api/v1/sync/outbox", post(ingest_event))
        .route("/api/v1/sync/products", get(products))
        .route("/api/v1/sync/users", get(sync_users))
        .route(
            "/api/v1/updates/{target}/{arch}/{version}",
            get(desktop_update),
        )
        .route("/api/v1/reports/sales-by-unit", get(sales_by_unit))
        .route("/api/v1/reports/sales", get(tenant_sales_report))
        .route("/api/v1/dashboard/summary", get(dashboard_summary))
        .route("/api/v1/owner/overview", get(owner_overview))
        .route(
            "/api/v1/owner/product-settings",
            post(save_unit_product_setting),
        )
        .route("/api/v1/owner/promotions", post(save_owner_promotion))
        .route(
            "/api/v1/owner/display-channels",
            post(create_display_channel),
        )
        .route(
            "/api/v1/owner/branding",
            get(owner_branding)
                .put(save_owner_branding)
                .post(save_owner_branding),
        )
        .route(
            "/api/v1/display/{token}/promotions",
            get(display_promotions),
        )
        .route("/tv/{token}", get(online_tv_page))
        .route("/assets/demo-products-v1.png", get(demo_products_sprite))
        .route("/assets/demo-product/{ean}", get(demo_product_image))
        .route("/api/v1/admin/licenses/issue", post(issue_license))
        .route("/api/v1/licenses/renew", post(renew_license))
        .route("/api/v1/payments/pix", post(create_pix_intent))
        .route("/api/v1/payments/{id}", get(payment_intent))
        .route("/api/v1/webhooks/asaas", post(asaas_webhook))
        .layer(TraceLayer::new_for_http())
        .with_state(state);
    // Render assigns the listening port at runtime through PORT.  Local Docker
    // and desktop development can still explicitly set BIND_ADDR.
    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| {
        let port = env::var("PORT").unwrap_or_else(|_| "8080".into());
        format!("0.0.0.0:{port}")
    });
    let addr: SocketAddr = bind_addr.parse()?;
    info!(%addr, "CommerceCTRL backend iniciado");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_every_relevant_asaas_status() {
        assert_eq!(asaas_status("RECEIVED"), "received");
        assert_eq!(asaas_status("CONFIRMED"), "received");
        assert_eq!(asaas_status("REFUNDED"), "refunded");
        assert_eq!(asaas_status("OVERDUE"), "overdue");
        assert_eq!(asaas_status("DELETED"), "cancelled");
        assert_eq!(asaas_status("PENDING"), "pending");
    }

    #[test]
    fn header_uuid_accepts_valid_and_rejects_missing_or_invalid() {
        let id = Uuid::new_v4();
        let mut headers = HeaderMap::new();
        headers.insert("x-id", id.to_string().parse().unwrap());
        assert_eq!(header_uuid(&headers, "x-id").unwrap(), id);
        assert_eq!(
            header_uuid(&headers, "missing").unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
        headers.insert("x-id", "invalid".parse().unwrap());
        assert!(header_uuid(&headers, "x-id").is_err());
    }

    #[test]
    fn key_hash_is_deterministic_and_does_not_leak_secret() {
        let first = hash_key("segredo");
        assert_eq!(first, hash_key("segredo"));
        assert_ne!(first, hash_key("outro"));
        assert_eq!(first.len(), 64);
        assert!(!first.contains("segredo"));
    }

    #[test]
    fn signed_license_has_verifiable_signature() {
        let claims = LicenseClaims {
            tenant_id: Uuid::new_v4().to_string(),
            unit_id: Uuid::new_v4().to_string(),
            installation_id: "terminal-1".into(),
            issued_at: 1,
            expires_at: 2,
            grace_until: 3,
            nonce: "nonce".into(),
        };
        let signed = sign_license(claims.clone()).unwrap();
        let signature = STANDARD.decode(signed.signature).unwrap();
        let signature = ed25519_dalek::Signature::from_slice(&signature).unwrap();
        let payload = serde_json::to_vec(&claims).unwrap();
        use ed25519_dalek::Verifier;
        assert!(signing_key()
            .unwrap()
            .verifying_key()
            .verify(&payload, &signature)
            .is_ok());
    }

    #[test]
    fn api_errors_keep_status_and_json_message() {
        let result = error(StatusCode::BAD_REQUEST, "inválido");
        assert_eq!(result.0, StatusCode::BAD_REQUEST);
        assert_eq!(result.1 .0["error"], "inválido");
        assert_eq!(default_valid_days(), 30);
        assert_eq!(default_grace_days(), 15);
    }
}
