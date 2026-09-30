import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface Product {
  id: number;
  ean: string;
  part_number: string;
  description: string;
  brand?: string | null;
  price_brl_cents: number;
  stock_qty: number;
  updated_at: number;
}

export interface SaleOut {
  sale_uuid: string;
  total_brl_cents: number;
  payment_method: string;
  items_count: number;
  outbox_id: number;
  created_at: number;
}

export interface PaymentIn {
  method: string;
  amount_brl_cents: number;
  extra?: Record<string, unknown>;
}

export interface PaymentResult {
  ok: boolean;
  method: string;
  authorization_code?: string | null;
  qr_data?: string | null;
  message?: string | null;
}

export interface SyncInfo {
  online: boolean;
  last_sync_ms: number;
  pending: number;
}

export interface CashStatus {
  open: boolean;
  session_id: number | null;
  opened_at: number | null;
  opening_brl_cents: number;
  sales_brl_cents: number;
  expected_brl_cents: number;
  closed_at: number | null;
  closing_brl_cents: number | null;
  difference_brl_cents: number;
}

export interface BarcodeEvent {
  code: string;
  at_ms: number;
}

export type PaymentMethod = "pix" | "cash" | "credit" | "debit";

export type PrintStatusRust = "Ok" | "Busy" | "Error" | "Offline";

export interface CashMovement {
  id: number;
  session_id: number;
  movement_type: string;
  amount_brl_cents: number;
  description?: string | null;
  operator?: string | null;
  created_at: number;
}

export interface CashCounted {
  cash_brl_cents: number;
  card_brl_cents: number;
  pix_brl_cents: number;
  cheque_brl_cents: number;
}

export interface FechamentoReport {
  session: CashStatus;
  movements: CashMovement[];
  sales_by_method: Record<string, unknown>;
  expected_total: number;
  counted_total: number;
  difference: number;
}

const API = "http://127.0.0.1:9001";

export function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function http<T>(path: string, init?: RequestInit): Promise<T> {
  try {
    const r = await fetch(`${API}${path}`, init);
    if (!r.ok) {
      if (r.status === 404) return undefined as unknown as T;
      throw new Error(`HTTP ${r.status}`);
    }
    return r.json() as Promise<T>;
  } catch (e) {
    if (String(e).includes("Failed to fetch") || String(e).includes("NetworkError")) {
      throw new Error("Servidor offline. Execute `npm run tauri dev` ou conecte ao backend.");
    }
    throw e;
  }
}

const STOCK_MAX = 500;
let browserStock = new Map<string, number>();

function stockSet(ean: string, qty: number) {
  if (browserStock.size >= STOCK_MAX && !browserStock.has(ean)) {
    const first = browserStock.keys().next().value;
    if (first !== undefined) browserStock.delete(first);
  }
  browserStock.set(ean, qty);
}

let browserCash: CashStatus = {
  open: false,
  session_id: null,
  opened_at: null,
  opening_brl_cents: 0,
  sales_brl_cents: 0,
  expected_brl_cents: 0,
  closed_at: null,
  closing_brl_cents: null,
  difference_brl_cents: 0,
};

function fakePix(cents: number): PaymentResult {
  const payload =
    "00020126580014BR.GOV.BCB.PIX0136contapdv@autocontrol.com.br52040000530398654" +
    `${String(cents).padStart(4, "0")}` +
    "5802BR5912COMMERCECTRL6009SAO PAULO6304A13F";
  return { ok: true, method: "pix", qr_data: payload, authorization_code: "browser-mock" };
}

export async function searchProduct(query: string): Promise<Product[]> {
  if (inTauri()) return invoke<Product[]>("search_product", { query });
  return http<Product[]>(`/api/products/search?q=${encodeURIComponent(query)}`);
}

export async function searchByEan(ean: string): Promise<Product | null> {
  if (inTauri()) return invoke<Product | null>("search_by_ean", { ean });
  const p = await http<Product>(`/api/products/${encodeURIComponent(ean)}`);
  return p ?? null;
}

export async function listProducts(limit = 200, offset = 0): Promise<Product[]> {
  if (inTauri()) return invoke<Product[]>("list_products", { limit, offset });
  const rows = await http<Product[]>(`/api/products?limit=${limit}&offset=${offset}`);
  for (const p of rows) if (!browserStock.has(p.ean)) stockSet(p.ean, p.stock_qty);
  return rows;
}

export async function adjustStock(ean: string, delta: number): Promise<number> {
  if (inTauri()) return invoke<number>("adjust_stock", { ean, delta });
  const cur = browserStock.get(ean) ?? 0;
  const next = Math.max(0, cur + delta);
  stockSet(ean, next);
  return next;
}

export async function syncNow(): Promise<number> {
  if (inTauri()) return invoke<number>("sync_now");
  return 0;
}

export async function syncStatus(): Promise<SyncInfo> {
  if (inTauri()) return invoke<SyncInfo>("sync_status");
  return { online: true, last_sync_ms: Date.now(), pending: 0 };
}

export async function generatePix(
  amountBrlCents: number,
  _extra: Record<string, unknown>
): Promise<PaymentResult> {
  if (inTauri()) return invoke<PaymentResult>("generate_pix", { amountBrlCents });
  return fakePix(amountBrlCents);
}

export async function recordSale(
  items: { ean: string; qty: number; price_brl_cents: number }[],
  payment: PaymentIn
): Promise<SaleOut> {
  if (inTauri()) {
    return invoke<SaleOut>("record_sale", { items, payment });
  }
  const total = items.reduce((s, i) => s + i.price_brl_cents * i.qty, 0);
  if (browserCash.open) {
    browserCash.sales_brl_cents += total;
    browserCash.expected_brl_cents = browserCash.opening_brl_cents + browserCash.sales_brl_cents;
  }
  return {
    sale_uuid: `browser-${Date.now()}`,
    total_brl_cents: total,
    payment_method: payment.method,
    items_count: items.length,
    outbox_id: 0,
    created_at: Math.floor(Date.now() / 1000),
  };
}

export async function emitBarcode(code: string): Promise<void> {
  if (inTauri()) {
    await invoke("emit_barcode", { code });
    return;
  }
  window.dispatchEvent(new CustomEvent("pdv-barcode", { detail: { code, at_ms: Date.now() } }));
}

export function onBarcode(cb: (code: string) => void): Promise<() => void> {
  if (inTauri()) {
    return listen<BarcodeEvent>("barcode-scanned", (e) => cb(e.payload.code));
  }
  const handler = (e: Event) => cb((e as CustomEvent<BarcodeEvent>).detail.code);
  window.addEventListener("pdv-barcode", handler);
  return Promise.resolve(() => window.removeEventListener("pdv-barcode", handler));
}

export async function cashStatus(): Promise<CashStatus> {
  if (inTauri()) return invoke<CashStatus>("cash_status");
  return { ...browserCash };
}

export async function cashOpen(openingBrlCents: number): Promise<CashStatus> {
  if (inTauri()) return invoke<CashStatus>("cash_open", { openingBrlCents });
  if (browserCash.open) return { ...browserCash };
  browserCash = {
    open: true,
    session_id: 1,
    opened_at: Date.now(),
    opening_brl_cents: openingBrlCents,
    sales_brl_cents: 0,
    expected_brl_cents: openingBrlCents,
    closed_at: null,
    closing_brl_cents: null,
    difference_brl_cents: 0,
  };
  return { ...browserCash };
}

export async function cashClose(closingBrlCents: number): Promise<CashStatus> {
  if (inTauri()) return invoke<CashStatus>("cash_close", { closingBrlCents });
  if (!browserCash.open) return { ...browserCash };
  const diff = closingBrlCents - browserCash.expected_brl_cents;
  browserCash = {
    ...browserCash,
    open: false,
    closed_at: Date.now(),
    closing_brl_cents: closingBrlCents,
    difference_brl_cents: diff,
  };
  return { ...browserCash };
}

// ── Print ────────────────────────────────────────────────────

export async function printReceipt(
  saleUuid: string,
  items: [string, number][],
  totalBrlCents: number
): Promise<void> {
  if (inTauri()) {
    await invoke("print_receipt", { saleUuid, items, totalBrlCents });
    return;
  }
}

export async function printText(text: string): Promise<void> {
  if (inTauri()) {
    await invoke("print_text", { text });
    return;
  }
}

export async function printStatus(): Promise<PrintStatusRust> {
  if (inTauri()) return invoke<PrintStatusRust>("print_status");
  return "Ok";
}

export async function printRetry(spoolId: number): Promise<void> {
  if (inTauri()) {
    await invoke("print_retry", { spoolId });
    return;
  }
}

// ── Cash Movements ───────────────────────────────────────────

export async function cashSangria(
  sessionId: number,
  amountBrlCents: number,
  description?: string
): Promise<CashMovement> {
  if (inTauri()) {
    return invoke<CashMovement>("cash_sangria", {
      sessionId,
      amountBrlCents,
      description: description ?? null,
      operator: null,
    });
  }
  return {
    id: Date.now(),
    session_id: sessionId,
    movement_type: "sangria",
    amount_brl_cents: amountBrlCents,
    description,
    created_at: Date.now(),
  };
}

export async function cashSuprimento(
  sessionId: number,
  amountBrlCents: number,
  description?: string
): Promise<CashMovement> {
  if (inTauri()) {
    return invoke<CashMovement>("cash_suprimento", {
      sessionId,
      amountBrlCents,
      description: description ?? null,
      operator: null,
    });
  }
  return {
    id: Date.now(),
    session_id: sessionId,
    movement_type: "suprimento",
    amount_brl_cents: amountBrlCents,
    description,
    created_at: Date.now(),
  };
}

export async function cashFechamentoCego(
  sessionId: number,
  counted: CashCounted
): Promise<FechamentoReport> {
  if (inTauri()) {
    return invoke<FechamentoReport>("cash_fechamento_cego", { sessionId, counted });
  }
  return {
    session: { ...browserCash },
    movements: [],
    sales_by_method: {},
    expected_total: browserCash.expected_brl_cents,
    counted_total:
      counted.cash_brl_cents +
      counted.card_brl_cents +
      counted.pix_brl_cents +
      counted.cheque_brl_cents,
    difference:
      counted.cash_brl_cents +
      counted.card_brl_cents +
      counted.pix_brl_cents +
      counted.cheque_brl_cents -
      browserCash.expected_brl_cents,
  };
}

// ── Customers (Fidelidade) ──────────────────────────────────

export interface Customer {
  id: number;
  name: string;
  cpf_cnpj?: string | null;
  phone?: string | null;
  email?: string | null;
  points: number;
  total_spent_brl_cents: number;
  created_at: number;
}

export async function searchCustomers(query: string): Promise<Customer[]> {
  if (inTauri()) return invoke<Customer[]>("search_customers", { query });
  return [];
}

export async function listCustomers(limit = 50): Promise<Customer[]> {
  if (inTauri()) return invoke<Customer[]>("list_customers", { limit });
  return [];
}

export async function upsertCustomer(
  name: string,
  cpfCnpj?: string,
  phone?: string,
  email?: string,
): Promise<Customer> {
  if (inTauri()) {
    return invoke<Customer>("upsert_customer", {
      name,
      cpfCnpj: cpfCnpj ?? null,
      phone: phone ?? null,
      email: email ?? null,
    });
  }
  return { id: Date.now(), name, cpf_cnpj: cpfCnpj, points: 0, total_spent_brl_cents: 0, created_at: Date.now() };
}

export async function addCustomerPoints(
  customerId: number,
  points: number,
  saleUuid?: string,
  description?: string,
): Promise<void> {
  if (inTauri()) {
    await invoke("add_customer_points", {
      customerId,
      points,
      saleUuid: saleUuid ?? null,
      description: description ?? null,
    });
  }
}

export async function redeemCustomerPoints(
  customerId: number,
  points: number,
  description?: string,
): Promise<number> {
  if (inTauri()) {
    return invoke<number>("redeem_customer_points", {
      customerId,
      points,
      description: description ?? null,
    });
  }
  return 0;
}

// ── NFe B2B ─────────────────────────────────────────────────

export interface NfeItem {
  ean: string;
  description: string;
  ncm: string;
  cfop: string;
  unit: string;
  qty: number;
  unit_value_brl_cents: number;
  total_brl_cents: number;
}

export interface NfeParams {
  modelo: number;
  serie: number;
  numero: number;
  cnpj_emitente: string;
  cnpj_destinatario: string;
  uf: string;
  cfop: string;
  items: NfeItem[];
}

export interface NfeResult {
  xml: string;
  chave: string;
  emissao: string;
  error?: string | null;
}

export async function emitNfe(params: NfeParams): Promise<NfeResult> {
  if (inTauri()) return invoke<NfeResult>("emit_nfe", { params });
  return { xml: "", chave: "", emissao: "Normal", error: "Disponível apenas no PDV desktop" };
}
