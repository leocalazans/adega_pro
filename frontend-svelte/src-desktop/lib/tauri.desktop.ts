import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  CashClosingReport,
  CashStatus,
  CloudPaymentIntent,
  Customer,
  DashboardSummary,
  DisplaySettings,
  Employee,
  Expense,
  KioskAdminStatus,
  LicenseStatus,
  LocalDisplayLink,
  OwnerOverview,
  PaymentResult,
  PaymentTotals,
  Product,
  Promotion,
  SaleOut,
  SalesReportRow,
  Supplier,
  SyncInfo,
  TenantBranding,
  TimeEntry,
  UnitSalesReport,
  UserSession,
  LoginResult,
  ActivationStatus,
} from "../../src-shared/types";

export async function loginEmployee(username: string, password: string): Promise<LoginResult> {
  return invoke("login_employee", { username, password });
}
export async function currentSession(): Promise<UserSession | null> {
  return invoke("current_session");
}
export async function logoutEmployee(): Promise<void> {
  await invoke("logout_employee");
}
export async function changeOwnPassword(currentPassword: string, newPassword: string): Promise<void> {
  await invoke("change_own_password", { currentPassword, newPassword });
}
export async function createTerminalBackup(): Promise<string> {
  return invoke("create_terminal_backup");
}
export async function scheduleTerminalRestore(path: string): Promise<void> {
  await invoke("schedule_terminal_restore", { path });
}

export const inTauri = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export async function kioskAdminStatus(): Promise<KioskAdminStatus> {
  return invoke("kiosk_admin_status");
}
export async function configureKioskAdminPin(pin: string): Promise<void> {
  await invoke("configure_kiosk_admin_pin", { pin });
}
export async function exitKioskAsAdmin(pin: string): Promise<void> {
  await invoke("exit_kiosk_as_admin", { pin });
}
export async function verifyAdminPin(pin: string): Promise<void> {
  await invoke("verify_admin_pin", { pin });
}

export async function searchProduct(query: string): Promise<Product[]> {
  return invoke("search_product", { query });
}
export async function searchByEan(ean: string): Promise<Product | null> {
  return invoke("search_by_ean", { ean });
}
export async function listProducts(limit = 200, offset = 0): Promise<Product[]> {
  return invoke("list_products", { limit, offset });
}
export async function saveProduct(input: Omit<Product, "id" | "updated_at" | "active">): Promise<Product> {
  return invoke("save_product", {
    ean: input.ean,
    partNumber: input.part_number,
    description: input.description,
    brand: input.brand || null,
    priceBrlCents: input.price_brl_cents,
    stockQty: input.stock_qty,
    minStock: input.min_stock,
    imageUrl: input.image_url || null,
  });
}
export async function createPurchaseOrder(ean: string, quantity: number): Promise<number> {
  return invoke("create_purchase_order", { ean, quantity });
}
export async function archiveProduct(ean: string): Promise<void> {
  return invoke("archive_product", { ean });
}

export async function recordSale(
  items: { ean: string; qty: number; price_brl_cents: number }[],
  payment: { method: string; amount_brl_cents: number; extra?: Record<string, unknown> },
  terminalId = "default",
  consumerDocument = "",
): Promise<SaleOut> {
  return invoke("record_sale", {
    items,
    payment,
    terminalId,
    consumerDocument: consumerDocument || null,
  });
}
export async function generatePix(amountBrlCents: number, pixKey: string): Promise<PaymentResult> {
  return invoke("generate_pix", {
    amountBrlCents,
    extra: { pix_key: pixKey, merchant_name: "COMMERCECTRL", merchant_city: "SAO PAULO" },
  });
}
export async function cloudGeneratePix(amountBrlCents: number): Promise<CloudPaymentIntent> {
  return invoke("cloud_generate_pix", { amountBrlCents });
}
export async function cloudPaymentStatus(id: string): Promise<CloudPaymentIntent> {
  return invoke("cloud_payment_status", { id });
}

export async function cashStatus(): Promise<CashStatus> {
  return invoke("cash_status");
}
export async function cashOpen(openingBrlCents: number): Promise<CashStatus> {
  return invoke("cash_open", { openingBrlCents });
}
export async function cashClose(closingBrlCents: number): Promise<CashStatus> {
  return invoke("cash_close", { closingBrlCents });
}
export async function cashSangria(sessionId: number, amountBrlCents: number, description: string, operator: string): Promise<void> {
  await invoke("cash_sangria", { sessionId, amountBrlCents, description: description || null, operator: operator || null });
}
export async function cashSuprimento(sessionId: number, amountBrlCents: number, description: string, operator: string): Promise<void> {
  await invoke("cash_suprimento", { sessionId, amountBrlCents, description: description || null, operator: operator || null });
}
export async function cashBlindClosing(sessionId: number, cash: number, card: number, pix: number): Promise<CashClosingReport> {
  return invoke("cash_fechamento_cego", {
    sessionId,
    counted: { cash_brl_cents: cash, card_brl_cents: card, pix_brl_cents: pix, cheque_brl_cents: 0 },
  });
}

export async function syncStatus(): Promise<SyncInfo> {
  return invoke("sync_status");
}
export function onBarcode(callback: (code: string) => void): Promise<() => void> {
  return listen<{ code: string }>("barcode-scanned", (event) => callback(event.payload.code));
}
export async function printReceipt(
  saleUuid: string,
  items: [string, number][],
  totalBrlCents: number,
  storeName?: string,
  paymentMethod?: string,
  consumerDocument?: string,
  logoDataUrl?: string,
) {
  const mode = await getPrinterConfig().mode;
  if (mode === "windows_graphic") {
    printGraphicalReceipt(saleUuid, items, totalBrlCents, storeName, paymentMethod, consumerDocument, logoDataUrl);
    return;
  }
  await invoke("print_receipt", {
    saleUuid,
    items,
    totalBrlCents,
    storeName: storeName || null,
    paymentMethod: paymentMethod || null,
    consumerDocument: consumerDocument || null,
    logoDataUrl: logoDataUrl || null,
  });
}
function printGraphicalReceipt(saleUuid: string, items: [string, number][], total: number, storeName?: string, payment?: string, consumerDoc?: string, logo?: string) {
  const escape = (value: string) => value.replace(/[&<>"']/g, (char) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[char]!));
  const money = (value: number) => new Intl.NumberFormat("pt-BR", { style: "currency", currency: "BRL" }).format(value / 100);
  const frame = document.createElement("iframe");
  frame.style.cssText = "position:fixed;width:0;height:0;border:0";
  document.body.appendChild(frame);
  const doc = frame.contentDocument!;
  doc.open();
  doc.write(`<!doctype html><meta charset="utf-8"><title>Comprovante</title><style>@page{margin:8mm}body{font:12px Arial;color:#000;max-width:78mm;margin:auto}header{text-align:center}img{max-width:42mm;max-height:22mm;object-fit:contain}h1{font-size:17px;margin:5px 0}.warning{font-weight:bold;border:1px solid;padding:5px}.row{display:flex;justify-content:space-between;border-bottom:1px dashed #999;padding:5px 0}.total{font-size:20px;font-weight:bold;margin-top:10px;text-align:right}small{display:block;margin-top:7px}</style><header>${logo ? `<img src="${logo}"/>` : ""}<h1>${escape(storeName || "ESTABELECIMENTO")}</h1><div class="warning">COMPROVANTE DE VENDA<br>NÃO É DOCUMENTO FISCAL</div></header>${items.map(([name, price]) => `<div class="row"><span>${escape(name)}</span><b>${money(price)}</b></div>`).join("")}<div class="total">TOTAL ${money(total)}</div><small>Venda: ${escape(saleUuid)}</small><small>Pagamento: ${escape(payment || "não informado")}</small>${consumerDoc ? `<small>Consumidor: ${escape(consumerDoc)}</small>` : ""}<small>${new Date().toLocaleString("pt-BR")}</small>`);
  doc.close();
  setTimeout(() => { frame.contentWindow?.focus(); frame.contentWindow?.print(); setTimeout(() => frame.remove(), 30_000), 250 });
}
export type PrinterInfo = { name: string; is_default: boolean; is_network: boolean };
export type PrinterConfig = {
  mode: "auto" | "windows" | "windows_graphic" | "escpos_network" | "local_port" | "smartpos";
  windows_printer: string | null;
  escpos_host: string | null;
  escpos_port: number;
  local_port: string | null;
  prefer_escpos: boolean;
  print_logo: boolean;
};
export async function listPrinters(): Promise<PrinterInfo[]> {
  return invoke("list_printers");
}
export async function getPrinterConfig(): Promise<PrinterConfig> {
  return invoke("printer_config");
}
export async function savePrinterConfig(config: PrinterConfig): Promise<void> {
  await invoke("save_printer_config", { config });
}
export async function testPrinter(): Promise<void> {
  if ((await getPrinterConfig()).mode === "windows_graphic") {
    printGraphicalReceipt("TESTE", [["Teste de impressão", 0]], 0, "CommerceCTRL", "teste");
    return;
  }
  await invoke("print_text", { text: "COMMERCECTRL\nTeste de impressao concluido\n" });
}

export async function dashboardSummary(): Promise<DashboardSummary> {
  return invoke("dashboard_summary");
}
export async function listEmployees(): Promise<Employee[]> {
  return invoke("list_employees");
}
export async function listTimeEntries(limit = 200): Promise<TimeEntry[]> {
  return invoke("list_time_entries", { limit });
}
export async function recordTimeEntry(employeeId: number, eventType: TimeEntry["event_type"], note = ""): Promise<TimeEntry> {
  return invoke("record_time_entry", { employeeId, eventType, note: note || null });
}
export async function saveEmployee(name: string, role: string, status: string): Promise<Employee> {
  return invoke("save_employee", { name, role, status });
}
export async function provisionEmployeeLogin(employeeId: number, username: string, role: string, temporaryPassword: string): Promise<void> {
  await invoke("provision_employee_login", { employeeId, username, role, temporaryPassword });
}
export async function updateEmployee(id: number, name: string, role: string, status: string): Promise<Employee> {
  return invoke("update_employee", { id, name, role, status });
}
export async function setEmployeeStatus(id: number, status: string): Promise<void> {
  await invoke("set_employee_status", { id, status });
}
export async function listExpenses(): Promise<Expense[]> {
  return invoke("list_expenses");
}
export async function saveExpense(description: string, category: string, amountBrlCents: number, dueDate: string, status: string): Promise<Expense> {
  return invoke("save_expense", { description, category, amountBrlCents, dueDate, status });
}
export async function updateExpense(id: number, description: string, category: string, amountBrlCents: number, dueDate: string, status: string): Promise<Expense> {
  return invoke("update_expense", { id, description, category, amountBrlCents, dueDate, status });
}
export async function setExpenseStatus(id: number, status: string): Promise<void> {
  await invoke("set_expense_status", { id, status });
}
export async function listSuppliers(): Promise<Supplier[]> {
  return invoke("list_suppliers");
}
export async function saveSupplier(name: string, document: string, phone: string, status: string): Promise<Supplier> {
  return invoke("save_supplier", { name, document: document || null, phone: phone || null, status });
}
export async function updateSupplier(id: number, name: string, document: string, phone: string, status: string): Promise<Supplier> {
  return invoke("update_supplier", { id, name, document: document || null, phone: phone || null, status });
}
export async function setSupplierStatus(id: number, status: string): Promise<void> {
  await invoke("set_supplier_status", { id, status });
}
export async function listPromotions(includeInactive = false): Promise<Promotion[]> {
  return invoke("list_promotions", { includeInactive });
}
export async function setPromotionActive(id: number, active: boolean): Promise<void> {
  await invoke("set_promotion_active", { id, active });
}
export async function savePromotion(title: string, subtitle: string, priceLabel: string, active: boolean): Promise<Promotion> {
  return invoke("save_promotion", { title, subtitle: subtitle || null, priceLabel, active });
}
export async function updatePromotion(id: number, title: string, subtitle: string, priceLabel: string, active: boolean): Promise<Promotion> {
  return invoke("update_promotion", { id, title, subtitle: subtitle || null, priceLabel, active });
}
export async function listCustomers(limit = 100): Promise<Customer[]> {
  return invoke("list_customers", { limit });
}
export async function saveCustomer(name: string, cpfCnpj: string, phone: string, email: string): Promise<Customer> {
  return invoke("upsert_customer", { name, cpfCnpj: cpfCnpj || null, phone: phone || null, email: email || null });
}
export async function updateCustomer(id: number, name: string, cpfCnpj: string, phone: string, email: string): Promise<Customer> {
  return invoke("update_customer", { id, name, cpfCnpj: cpfCnpj || null, phone: phone || null, email: email || null });
}
export async function currentPaymentTotals(): Promise<PaymentTotals> {
  return invoke("current_payment_totals");
}
export async function salesReport(from: number, to: number, paymentMethod?: string, terminalId?: string, productQuery?: string): Promise<SalesReportRow[]> {
  return invoke("sales_report", { from, to, paymentMethod: paymentMethod || null, terminalId: terminalId || null, productQuery: productQuery || null });
}
export async function cloudSalesByUnit(from: number, to: number): Promise<UnitSalesReport[]> {
  return invoke("cloud_sales_by_unit", { from, to });
}
export async function cloudOwnerOverview(from: number, to: number): Promise<OwnerOverview> {
  return invoke("cloud_owner_overview", { from, to });
}
export async function cloudOwnerBranding(): Promise<TenantBranding> {
  return invoke("cloud_owner_branding");
}
export async function cloudSaveOwnerBranding(displayName: string, logoDataUrl: string, primaryColor?: string | null): Promise<TenantBranding> {
  return invoke("cloud_save_owner_branding", { displayName, logoDataUrl: logoDataUrl || null, primaryColor: primaryColor || null });
}
export async function localDisplayLink(): Promise<LocalDisplayLink> {
  return invoke("local_display_link");
}
export async function getDisplaySettings(): Promise<DisplaySettings> {
  return invoke("get_display_settings");
}
export async function saveDisplaySettings(settings: DisplaySettings): Promise<DisplaySettings> {
  return invoke("save_display_settings", { settings });
}
export async function licenseStatus(): Promise<LicenseStatus> {
  return invoke("license_status");
}
export async function scanLicenseMedia(): Promise<LicenseStatus> {
  return invoke("scan_license_media");
}
export async function importLicense(path: string): Promise<LicenseStatus> {
  return invoke("import_license", { path });
}
export async function refreshLicense(): Promise<LicenseStatus> {
  return invoke("refresh_license");
}
export async function activationStatus(): Promise<ActivationStatus> {
  return invoke("activation_status");
}
export async function claimActivation(code: string, terminalName: string, apiUrl?: string): Promise<ActivationStatus> {
  return invoke("claim_activation", { request: { code, terminal_name: terminalName, api_url: apiUrl || null } });
}


