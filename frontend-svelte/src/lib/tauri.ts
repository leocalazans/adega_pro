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
} from "./types";

export async function loginEmployee(username:string,password:string):Promise<LoginResult>{return invoke("login_employee",{username,password})}
export async function currentSession():Promise<UserSession|null>{return inTauri()?invoke("current_session"):null}
export async function logoutEmployee():Promise<void>{if(inTauri())await invoke("logout_employee")}
export async function changeOwnPassword(currentPassword:string,newPassword:string):Promise<void>{if(inTauri())await invoke("change_own_password",{currentPassword,newPassword})}
export async function createTerminalBackup():Promise<string>{return invoke("create_terminal_backup")}
export async function scheduleTerminalRestore(path:string):Promise<void>{await invoke("schedule_terminal_restore",{path})}

export const inTauri = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const LOCAL_API = "http://127.0.0.1:9001";

export async function kioskAdminStatus(): Promise<KioskAdminStatus> {
  if (inTauri()) return invoke("kiosk_admin_status");
  return { configured: false };
}
export async function configureKioskAdminPin(pin: string): Promise<void> {
  if (inTauri()) return invoke("configure_kiosk_admin_pin", { pin });
  throw new Error("A configuração exige o aplicativo desktop");
}
export async function exitKioskAsAdmin(pin: string): Promise<void> {
  if (inTauri()) return invoke("exit_kiosk_as_admin", { pin });
  throw new Error("A saída protegida exige o aplicativo desktop");
}
export async function verifyAdminPin(pin: string): Promise<void> {
  if (inTauri()) return invoke("verify_admin_pin", { pin });
  throw new Error("A autorização exige o aplicativo desktop");
}

async function localHttp<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${LOCAL_API}${path}`, init);
  if (!response.ok)
    throw new Error(`Serviço local indisponível (HTTP ${response.status})`);
  return response.json() as Promise<T>;
}

export async function searchProduct(query: string): Promise<Product[]> {
  if (inTauri()) return invoke("search_product", { query });
  return localHttp(`/api/products/search?q=${encodeURIComponent(query)}`);
}

export async function searchByEan(ean: string): Promise<Product | null> {
  if (inTauri()) return invoke("search_by_ean", { ean });
  return localHttp(`/api/products/${encodeURIComponent(ean)}`);
}

export async function listProducts(
  limit = 200,
  offset = 0,
): Promise<Product[]> {
  if (inTauri()) return invoke("list_products", { limit, offset });
  return localHttp(`/api/products?limit=${limit}&offset=${offset}`);
}
export async function saveProduct(
  input: Omit<Product, "id" | "updated_at" | "active">,
): Promise<Product> {
  if (inTauri())
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
  throw new Error("Cadastro requer o aplicativo desktop");
}
export async function createPurchaseOrder(
  ean: string,
  quantity: number,
): Promise<number> {
  if (inTauri()) return invoke("create_purchase_order", { ean, quantity });
  throw new Error("Pedido de compra requer o aplicativo desktop");
}
export async function archiveProduct(ean: string): Promise<void> {
  if (inTauri()) return invoke("archive_product", { ean });
  throw new Error("Arquivamento requer o aplicativo desktop");
}

export async function recordSale(
  items: { ean: string; qty: number; price_brl_cents: number }[],
  payment: {
    method: string;
    amount_brl_cents: number;
    extra?: Record<string, unknown>;
  },
  terminalId = "default",
  consumerDocument = "",
): Promise<SaleOut> {
  if (inTauri())
    return invoke("record_sale", {
      items,
      payment,
      terminalId,
      consumerDocument: consumerDocument || null,
    });
  return localHttp("/api/sales", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      items,
      payment,
      terminal_id: terminalId,
      consumer_document: consumerDocument || null,
    }),
  });
}
export async function generatePix(
  amountBrlCents: number,
  pixKey: string,
): Promise<PaymentResult> {
  if (inTauri())
    return invoke("generate_pix", {
      amountBrlCents,
      extra: {
        pix_key: pixKey,
        merchant_name: "COMMERCECTRL",
        merchant_city: "SAO PAULO",
      },
    });
  throw new Error("Geração PIX requer o aplicativo desktop");
}
export async function cloudGeneratePix(
  amountBrlCents: number,
): Promise<CloudPaymentIntent> {
  if (inTauri()) return invoke("cloud_generate_pix", { amountBrlCents });
  throw new Error("PIX automático requer o aplicativo desktop");
}
export async function cloudPaymentStatus(
  id: string,
): Promise<CloudPaymentIntent> {
  if (inTauri()) return invoke("cloud_payment_status", { id });
  throw new Error("Consulta PIX requer o aplicativo desktop");
}

export async function cashStatus(): Promise<CashStatus> {
  if (inTauri()) return invoke("cash_status");
  throw new Error("Status de caixa requer o aplicativo desktop");
}
export async function cashOpen(openingBrlCents: number): Promise<CashStatus> {
  if (inTauri()) return invoke("cash_open", { openingBrlCents });
  throw new Error("Abertura de caixa requer o aplicativo desktop");
}

export async function cashClose(closingBrlCents: number): Promise<CashStatus> {
  if (inTauri()) return invoke("cash_close", { closingBrlCents });
  throw new Error(
    `Fechamento de ${closingBrlCents} requer o aplicativo desktop`,
  );
}
export async function cashSangria(
  sessionId: number,
  amountBrlCents: number,
  description: string,
  operator: string,
): Promise<void> {
  if (inTauri())
    return invoke("cash_sangria", {
      sessionId,
      amountBrlCents,
      description: description || null,
      operator: operator || null,
    });
  throw new Error("Sangria requer o aplicativo desktop");
}
export async function cashSuprimento(
  sessionId: number,
  amountBrlCents: number,
  description: string,
  operator: string,
): Promise<void> {
  if (inTauri())
    return invoke("cash_suprimento", {
      sessionId,
      amountBrlCents,
      description: description || null,
      operator: operator || null,
    });
  throw new Error("Suprimento requer o aplicativo desktop");
}
export async function cashBlindClosing(
  sessionId: number,
  cash: number,
  card: number,
  pix: number,
): Promise<CashClosingReport> {
  if (inTauri())
    return invoke("cash_fechamento_cego", {
      sessionId,
      counted: {
        cash_brl_cents: cash,
        card_brl_cents: card,
        pix_brl_cents: pix,
        cheque_brl_cents: 0,
      },
    });
  throw new Error("Fechamento requer o aplicativo desktop");
}

export async function syncStatus(): Promise<SyncInfo> {
  if (inTauri()) return invoke("sync_status");
  return localHttp("/api/sync/status");
}

export function onBarcode(
  callback: (code: string) => void,
): Promise<() => void> {
  if (inTauri())
    return listen<{ code: string }>("barcode-scanned", (event) =>
      callback(event.payload.code),
    );
  return Promise.resolve(() => undefined);
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
  if (inTauri() && (await getPrinterConfig()).mode === "windows_graphic") {
    printGraphicalReceipt(saleUuid, items, totalBrlCents, storeName, paymentMethod, consumerDocument, logoDataUrl);
    return;
  }
  if (inTauri())
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
function printGraphicalReceipt(saleUuid:string,items:[string,number][],total:number,storeName?:string,payment?:string,consumerDoc?:string,logo?:string){
  const escape=(value:string)=>value.replace(/[&<>"']/g,char=>({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"}[char]!));
  const money=(value:number)=>new Intl.NumberFormat("pt-BR",{style:"currency",currency:"BRL"}).format(value/100);
  const frame=document.createElement("iframe");frame.style.cssText="position:fixed;width:0;height:0;border:0";document.body.appendChild(frame);
  const doc=frame.contentDocument!;doc.open();doc.write(`<!doctype html><meta charset="utf-8"><title>Comprovante</title><style>@page{margin:8mm}body{font:12px Arial;color:#000;max-width:78mm;margin:auto}header{text-align:center}img{max-width:42mm;max-height:22mm;object-fit:contain}h1{font-size:17px;margin:5px 0}.warning{font-weight:bold;border:1px solid;padding:5px}.row{display:flex;justify-content:space-between;border-bottom:1px dashed #999;padding:5px 0}.total{font-size:20px;font-weight:bold;margin-top:10px;text-align:right}small{display:block;margin-top:7px}</style><header>${logo?`<img src="${logo}"/>`:""}<h1>${escape(storeName||"ESTABELECIMENTO")}</h1><div class="warning">COMPROVANTE DE VENDA<br>NÃO É DOCUMENTO FISCAL</div></header>${items.map(([name,price])=>`<div class="row"><span>${escape(name)}</span><b>${money(price)}</b></div>`).join("")}<div class="total">TOTAL ${money(total)}</div><small>Venda: ${escape(saleUuid)}</small><small>Pagamento: ${escape(payment||"não informado")}</small>${consumerDoc?`<small>Consumidor: ${escape(consumerDoc)}</small>`:""}<small>${new Date().toLocaleString("pt-BR")}</small>`);doc.close();
  setTimeout(()=>{frame.contentWindow?.focus();frame.contentWindow?.print();setTimeout(()=>frame.remove(),30_000)},250);
}
export type PrinterInfo = {
  name: string;
  is_default: boolean;
  is_network: boolean;
};
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
  return inTauri() ? invoke("list_printers") : [];
}
export async function getPrinterConfig(): Promise<PrinterConfig> {
  return inTauri()
    ? invoke("printer_config")
    : {
        mode: "auto",
        windows_printer: null,
        escpos_host: null,
        escpos_port: 9100,
        local_port: null,
        prefer_escpos: true,
        print_logo: true,
      };
}
export async function savePrinterConfig(config: PrinterConfig): Promise<void> {
  if (inTauri()) await invoke("save_printer_config", { config });
}
export async function testPrinter(): Promise<void> {
  if(inTauri() && (await getPrinterConfig()).mode === "windows_graphic"){
    printGraphicalReceipt("TESTE",[["Teste de impressão",0]],0,"CommerceCTRL","teste");return;
  }
  if (inTauri())
    await invoke("print_text", {
      text: "COMMERCECTRL\nTeste de impressao concluido\n",
    });
}

export async function dashboardSummary(): Promise<DashboardSummary> {
  if (inTauri()) return invoke("dashboard_summary");
  throw new Error("Dashboard requer o serviço local do aplicativo desktop");
}
export async function listEmployees(): Promise<Employee[]> {
  if (inTauri()) return invoke("list_employees");
  throw new Error(
    "Funcionários requerem o serviço local do aplicativo desktop",
  );
}
export async function listTimeEntries(limit = 200): Promise<TimeEntry[]> {
  if (inTauri()) return invoke("list_time_entries", { limit });
  throw new Error("Ponto requer o aplicativo desktop");
}
export async function recordTimeEntry(
  employeeId: number,
  eventType: TimeEntry["event_type"],
  note = "",
): Promise<TimeEntry> {
  if (inTauri()) return invoke("record_time_entry", { employeeId, eventType, note: note || null });
  throw new Error("Registro de ponto requer o aplicativo desktop");
}
export async function saveEmployee(
  name: string,
  role: string,
  status: string,
): Promise<Employee> {
  if (inTauri()) return invoke("save_employee", { name, role, status });
  throw new Error("Cadastro requer o aplicativo desktop");
}
export async function provisionEmployeeLogin(employeeId:number,username:string,role:string,temporaryPassword:string):Promise<void>{
  if(inTauri()) return invoke("provision_employee_login",{employeeId,username,role,temporaryPassword});
  throw new Error("Criação de acesso requer o aplicativo desktop");
}
export async function updateEmployee(
  id: number,
  name: string,
  role: string,
  status: string,
): Promise<Employee> {
  if (inTauri()) return invoke("update_employee", { id, name, role, status });
  throw new Error("Alteração requer o aplicativo desktop");
}
export async function setEmployeeStatus(
  id: number,
  status: string,
): Promise<void> {
  if (inTauri()) return invoke("set_employee_status", { id, status });
  throw new Error("Alteração requer o aplicativo desktop");
}
export async function listExpenses(): Promise<Expense[]> {
  if (inTauri()) return invoke("list_expenses");
  throw new Error("Despesas requerem o serviço local do aplicativo desktop");
}
export async function saveExpense(
  description: string,
  category: string,
  amountBrlCents: number,
  dueDate: string,
  status: string,
): Promise<Expense> {
  if (inTauri())
    return invoke("save_expense", {
      description,
      category,
      amountBrlCents,
      dueDate,
      status,
    });
  throw new Error("Cadastro requer o aplicativo desktop");
}
export async function updateExpense(
  id: number,
  description: string,
  category: string,
  amountBrlCents: number,
  dueDate: string,
  status: string,
): Promise<Expense> {
  if (inTauri())
    return invoke("update_expense", {
      id,
      description,
      category,
      amountBrlCents,
      dueDate,
      status,
    });
  throw new Error("Alteração requer o aplicativo desktop");
}
export async function setExpenseStatus(
  id: number,
  status: string,
): Promise<void> {
  if (inTauri()) return invoke("set_expense_status", { id, status });
  throw new Error("Alteração requer o aplicativo desktop");
}
export async function listSuppliers(): Promise<Supplier[]> {
  if (inTauri()) return invoke("list_suppliers");
  throw new Error(
    "Fornecedores requerem o serviço local do aplicativo desktop",
  );
}
export async function saveSupplier(
  name: string,
  document: string,
  phone: string,
  status: string,
): Promise<Supplier> {
  if (inTauri())
    return invoke("save_supplier", {
      name,
      document: document || null,
      phone: phone || null,
      status,
    });
  throw new Error("Cadastro requer o aplicativo desktop");
}
export async function updateSupplier(
  id: number,
  name: string,
  document: string,
  phone: string,
  status: string,
): Promise<Supplier> {
  if (inTauri())
    return invoke("update_supplier", {
      id,
      name,
      document: document || null,
      phone: phone || null,
      status,
    });
  throw new Error("Alteração requer o aplicativo desktop");
}
export async function setSupplierStatus(
  id: number,
  status: string,
): Promise<void> {
  if (inTauri()) return invoke("set_supplier_status", { id, status });
  throw new Error("Alteração requer o aplicativo desktop");
}
export async function listPromotions(
  includeInactive = false,
): Promise<Promotion[]> {
  if (inTauri()) return invoke("list_promotions", { includeInactive });
  if (includeInactive)
    throw new Error("A prévia web mostra somente promoções ativas");
  return localHttp("/api/display/promotions");
}
export async function setPromotionActive(
  id: number,
  active: boolean,
): Promise<void> {
  if (inTauri()) return invoke("set_promotion_active", { id, active });
  throw new Error("Alteração requer o aplicativo desktop");
}
export async function savePromotion(
  title: string,
  subtitle: string,
  priceLabel: string,
  active: boolean,
): Promise<Promotion> {
  if (inTauri())
    return invoke("save_promotion", {
      title,
      subtitle: subtitle || null,
      priceLabel,
      active,
    });
  throw new Error("Cadastro requer o aplicativo desktop");
}
export async function updatePromotion(
  id: number,
  title: string,
  subtitle: string,
  priceLabel: string,
  active: boolean,
): Promise<Promotion> {
  if (inTauri())
    return invoke("update_promotion", {
      id,
      title,
      subtitle: subtitle || null,
      priceLabel,
      active,
    });
  throw new Error("Alteração requer o aplicativo desktop");
}
export async function listCustomers(limit = 100): Promise<Customer[]> {
  if (inTauri()) return invoke("list_customers", { limit });
  throw new Error("Clientes requerem o serviço local do aplicativo desktop");
}
export async function saveCustomer(
  name: string,
  cpfCnpj: string,
  phone: string,
  email: string,
): Promise<Customer> {
  if (inTauri())
    return invoke("upsert_customer", {
      name,
      cpfCnpj: cpfCnpj || null,
      phone: phone || null,
      email: email || null,
    });
  throw new Error("Cadastro requer o aplicativo desktop");
}
export async function updateCustomer(
  id: number,
  name: string,
  cpfCnpj: string,
  phone: string,
  email: string,
): Promise<Customer> {
  if (inTauri())
    return invoke("update_customer", {
      id,
      name,
      cpfCnpj: cpfCnpj || null,
      phone: phone || null,
      email: email || null,
    });
  throw new Error("Alteração requer o aplicativo desktop");
}
export async function currentPaymentTotals(): Promise<PaymentTotals> {
  if (inTauri()) return invoke("current_payment_totals");
  throw new Error("Totais do caixa requerem o aplicativo desktop");
}
export async function salesReport(
  from: number,
  to: number,
  paymentMethod?: string,
  terminalId?: string,
  productQuery?: string,
): Promise<SalesReportRow[]> {
  if (inTauri())
    return invoke("sales_report", {
      from,
      to,
      paymentMethod: paymentMethod || null,
      terminalId: terminalId || null,
      productQuery: productQuery || null,
    });
  throw new Error("Relatório detalhado requer o aplicativo desktop");
}
export async function cloudSalesByUnit(
  from: number,
  to: number,
): Promise<UnitSalesReport[]> {
  if (inTauri()) return invoke("cloud_sales_by_unit", { from, to });
  throw new Error("Relatório consolidado requer o aplicativo desktop");
}
export async function cloudOwnerOverview(
  from: number,
  to: number,
): Promise<OwnerOverview> {
  if (inTauri()) return invoke("cloud_owner_overview", { from, to });
  throw new Error(
    "Painel do proprietário requer o aplicativo desktop autenticado",
  );
}
export async function cloudOwnerBranding(): Promise<TenantBranding> {
  if (inTauri()) return invoke("cloud_owner_branding");
  throw new Error("Identidade cloud requer o aplicativo autenticado");
}
export async function cloudSaveOwnerBranding(
  displayName: string,
  logoDataUrl: string,
): Promise<TenantBranding> {
  if (inTauri())
    return invoke("cloud_save_owner_branding", {
      displayName,
      logoDataUrl: logoDataUrl || null,
    });
  throw new Error("Alteração cloud requer o aplicativo autenticado");
}
export async function localDisplayLink(): Promise<LocalDisplayLink> {
  if (inTauri()) return invoke("local_display_link");
  throw new Error("Link da TV requer o aplicativo desktop da unidade");
}
export async function getDisplaySettings(): Promise<DisplaySettings> {
  if (inTauri()) return invoke("get_display_settings");
  return localHttp("/api/display/settings");
}
export async function saveDisplaySettings(
  settings: DisplaySettings,
): Promise<DisplaySettings> {
  if (inTauri()) return invoke("save_display_settings", { settings });
  throw new Error("Configuração da TV requer o aplicativo desktop da unidade");
}
export async function licenseStatus(): Promise<LicenseStatus> {
  if (inTauri()) return invoke("license_status");
  throw new Error("Licenciamento requer o aplicativo desktop");
}
export async function scanLicenseMedia(): Promise<LicenseStatus> {
  if (inTauri()) return invoke("scan_license_media");
  throw new Error("Leitura de pendrive requer o aplicativo desktop");
}
export async function importLicense(path: string): Promise<LicenseStatus> {
  if (inTauri()) return invoke("import_license", { path });
  throw new Error("Importação de licença requer o aplicativo desktop");
}
export async function refreshLicense(): Promise<LicenseStatus> {
  if (inTauri()) return invoke("refresh_license");
  throw new Error("Renovação requer o aplicativo desktop");
}
export async function activationStatus(): Promise<ActivationStatus> { return inTauri() ? invoke("activation_status") : { activated: true }; }
export async function claimActivation(code:string, terminalName:string, apiUrl?:string): Promise<ActivationStatus> {
  if (!inTauri()) throw new Error("A ativação exige o aplicativo desktop");
  return invoke("claim_activation", { request: { code, terminal_name: terminalName, api_url: apiUrl || null } });
}
