import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import {
  changeOwnPassword, configureKioskAdminPin, createTerminalBackup,
  currentSession, getPrinterConfig, inTauri, listPrinters, loginEmployee,
  logoutEmployee, provisionEmployeeLogin, savePrinterConfig,
  scheduleTerminalRestore, verifyAdminPin,
} from "./tauri";
import * as api from "./tauri";

describe("contrato IPC do desktop", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(undefined);
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
  });

  it("detecta o runtime Tauri", () => expect(inTauri()).toBe(true));

  it.each([
    ["login_employee", () => loginEmployee("caixa", "senha")],
    ["current_session", () => currentSession()],
    ["logout_employee", () => logoutEmployee()],
    ["change_own_password", () => changeOwnPassword("antiga", "nova-segura")],
    ["create_terminal_backup", () => createTerminalBackup()],
    ["schedule_terminal_restore", () => scheduleTerminalRestore("C:/backup.db")],
    ["configure_kiosk_admin_pin", () => configureKioskAdminPin("123456")],
    ["verify_admin_pin", () => verifyAdminPin("123456")],
    ["list_printers", () => listPrinters()],
    ["printer_config", () => getPrinterConfig()],
    ["save_printer_config", () => savePrinterConfig({mode:"auto",windows_printer:null,escpos_host:null,escpos_port:9100,local_port:null,prefer_escpos:true,print_logo:true})],
    ["provision_employee_login", () => provisionEmployeeLogin(1,"caixa","cashier","123456")],
  ])("encaminha %s sem substituir dados reais", async (command, call) => {
    await call();
    expect(invoke.mock.calls[invoke.mock.calls.length-1]?.[0]).toBe(command);
  });

  it("mantém o contrato de todos os comandos operacionais", async () => {
    const product={ean:"1",part_number:"SKU",description:"Produto",brand:null,price_brl_cents:100,stock_qty:1,min_stock:1,image_url:null};
    const display={layout:"promotions" as const,promotion_interval_seconds:10,youtube_id:null,selected_radio_url:null,radios:[]};
    const cases:[string,()=>Promise<unknown>][]=[
      ["kiosk_admin_status",()=>api.kioskAdminStatus()], ["exit_kiosk_as_admin",()=>api.exitKioskAsAdmin("123456")],
      ["search_product",()=>api.searchProduct("arroz")], ["search_by_ean",()=>api.searchByEan("1")], ["list_products",()=>api.listProducts()],
      ["save_product",()=>api.saveProduct(product)], ["create_purchase_order",()=>api.createPurchaseOrder("1",2)], ["archive_product",()=>api.archiveProduct("1")],
      ["record_sale",()=>api.recordSale([{ean:"1",qty:1,price_brl_cents:100}],{method:"cash",amount_brl_cents:100})],
      ["generate_pix",()=>api.generatePix(100,"ref")], ["cloud_generate_pix",()=>api.cloudGeneratePix(100)], ["cloud_payment_status",()=>api.cloudPaymentStatus("id")],
      ["cash_status",()=>api.cashStatus()], ["cash_open",()=>api.cashOpen(100)], ["cash_close",()=>api.cashClose(100)],
      ["cash_sangria",()=>api.cashSangria(1,10,"m","n")], ["cash_suprimento",()=>api.cashSuprimento(1,10,"m","n")],
      ["cash_fechamento_cego",()=>api.cashBlindClosing(1,1,2,3)], ["sync_status",()=>api.syncStatus()],
      ["dashboard_summary",()=>api.dashboardSummary()], ["list_employees",()=>api.listEmployees()], ["list_time_entries",()=>api.listTimeEntries()],
      ["record_time_entry",()=>api.recordTimeEntry(1,"clock_in","")], ["save_employee",()=>api.saveEmployee("A","Caixa","Ativo")],
      ["update_employee",()=>api.updateEmployee(1,"A","Caixa","Ativo")], ["set_employee_status",()=>api.setEmployeeStatus(1,"Ativo")],
      ["list_expenses",()=>api.listExpenses()], ["save_expense",()=>api.saveExpense("Luz","Fixa",100,"2026-01-01","Pago")],
      ["update_expense",()=>api.updateExpense(1,"Luz","Fixa",100,"2026-01-01","Pago")], ["set_expense_status",()=>api.setExpenseStatus(1,"Pago")],
      ["list_suppliers",()=>api.listSuppliers()], ["save_supplier",()=>api.saveSupplier("F","","","Ativo")],
      ["update_supplier",()=>api.updateSupplier(1,"F","","","Ativo")], ["set_supplier_status",()=>api.setSupplierStatus(1,"Ativo")],
      ["list_promotions",()=>api.listPromotions(true)], ["set_promotion_active",()=>api.setPromotionActive(1,true)],
      ["save_promotion",()=>api.savePromotion("Oferta","Sub","R$ 1",true)], ["update_promotion",()=>api.updatePromotion(1,"Oferta","Sub","R$ 1",true)],
      ["list_customers",()=>api.listCustomers()], ["upsert_customer",()=>api.saveCustomer("Cliente","52998224725","","")],
      ["update_customer",()=>api.updateCustomer(1,"Cliente","52998224725","","")], ["current_payment_totals",()=>api.currentPaymentTotals()],
      ["sales_report",()=>api.salesReport(0,1)], ["cloud_sales_by_unit",()=>api.cloudSalesByUnit(0,1)], ["cloud_owner_overview",()=>api.cloudOwnerOverview(0,1)],
      ["cloud_owner_branding",()=>api.cloudOwnerBranding()], ["cloud_save_owner_branding",()=>api.cloudSaveOwnerBranding("Loja","")],
      ["local_display_link",()=>api.localDisplayLink()], ["get_display_settings",()=>api.getDisplaySettings()], ["save_display_settings",()=>api.saveDisplaySettings(display)],
      ["license_status",()=>api.licenseStatus()], ["scan_license_media",()=>api.scanLicenseMedia()], ["import_license",()=>api.importLicense("x")], ["refresh_license",()=>api.refreshLicense()],
    ];
    for(const [command,call] of cases){invoke.mockClear();await call();expect(invoke.mock.calls[invoke.mock.calls.length-1]?.[0]).toBe(command)}
  });
});
