export interface Product {
  id: number;
  ean: string;
  part_number: string;
  description: string;
  brand?: string | null;
  price_brl_cents: number;
  stock_qty: number;
  updated_at: number;
  min_stock: number;
  active: boolean;
  image_url?: string | null;
}

export interface SaleOut {
  sale_uuid: string;
  total_brl_cents: number;
  payment_method: string;
  items_count: number;
  outbox_id: number;
  created_at: number;
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

export interface SyncInfo { online: boolean; last_sync_ms: number; pending: number; conflicts: number; last_error?: string | null }

export interface Employee { id: number; name: string; role: string; status: string }
export interface TimeEntry { id: number; employee_id: number; employee_name: string; event_type: "clock_in" | "break_start" | "break_end" | "clock_out"; note?: string | null; occurred_at: number }
export interface Expense { id: number; description: string; category: string; amount_brl_cents: number; due_date: string; status: string }
export interface Supplier { id: number; name: string; document?: string | null; phone?: string | null; status: string }
export interface Promotion { id: number; title: string; subtitle?: string | null; price_label: string; active: boolean }
export interface DashboardSummary { sales_today_brl_cents: number; items_today: number; new_customers_today: number; low_stock_count: number; daily_sales: number[] }
export interface Customer { id: number; name: string; cpf_cnpj?: string | null; phone?: string | null; email?: string | null; points: number; total_spent_brl_cents: number; created_at: number }
export interface PaymentTotals { cash_brl_cents: number; card_brl_cents: number; pix_brl_cents: number; other_brl_cents: number }
export interface SalesReportRow { uuid: string; total_brl_cents: number; payment_method: string; terminal_id: string; items_count: number; created_at: number }
export interface CashClosingReport { expected_total: number; counted_total: number; difference: number }
export interface PaymentResult { ok: boolean; method: string; authorization_code?: string | null; qr_data?: string | null; message?: string | null }
export interface UnitSalesReport { unit_id: string; unit_name: string; sales_count: number; total_brl_cents: number; items_count: number }
export interface OwnerUnitReport extends UnitSalesReport { unit_code: string; stock_skus: number; low_stock_skus: number }
export interface OwnerTopProductReport { ean: string; description: string; qty_sold: number; revenue_brl_cents: number; units_sold_in: number }
export interface OwnerOverview { total_brl_cents: number; sales_count: number; units: OwnerUnitReport[]; top_products: OwnerTopProductReport[] }
export interface TenantBranding { display_name: string; logo_data_url?: string | null }
export interface LocalDisplayLink { url: string; port: number; note: string }
export interface RadioStation { name: string; url: string }
export interface DisplaySettings { layout: "promotions" | "video"; youtube_id?: string | null; radios: RadioStation[]; selected_radio_url?: string | null; promotion_interval_seconds: number }
export interface LicenseStatus { allowed_to_sell: boolean; mode: string; installation_id: string; expires_at?: number | null; grace_until?: number | null; message: string }
export interface CloudPaymentIntent { id: string; provider: string; provider_id?: string | null; external_reference: string; amount_brl_cents: number; status: string; qr_payload?: string | null; expires_at?: string | null }
export interface KioskAdminStatus { configured: boolean }
export interface UserSession { user_id:number; employee_id?:number|null; display_name:string; username:string; role:string; permissions:string[] }
export interface LoginResult { session:UserSession; must_change_password:boolean }
export interface ActivationStatus { activated:boolean; api_url?:string|null }
