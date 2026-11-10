import type { TenantBranding } from "../../src-shared/types";

let _apiBaseOverride: string | null = null;

export function setApiBaseOverride(url: string | null): void {
  _apiBaseOverride = url;
}

export function getApiBase(): string {
  if (_apiBaseOverride) return _apiBaseOverride;
  if (typeof window !== "undefined") {
    const envUrl = (import.meta as any).env?.VITE_COMMERCECTRL_API_URL;
    if (envUrl) return envUrl.replace(/\/$/, "");
    return (window as any).__COMMERCECTRL_API_URL__ || "";
  }
  return "https://api.commercectrl.com";
}

export async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${getApiBase()}${path}`, {
    ...init,
    credentials: "include",
    headers: { "Content-Type": "application/json", ...(init?.headers ?? {}) },
  });
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new Error(text || `Falha na API (HTTP ${res.status})`);
  }
  return res.json() as Promise<T>;
}

export interface PlatformSession {
  email: string;
  role: string;
}

export async function platformLogin(email: string, password: string): Promise<PlatformSession> {
  return apiFetch<PlatformSession>("/api/v1/platform/auth/login", {
    method: "POST",
    body: JSON.stringify({ email, password }),
  });
}

export async function platformLogout(): Promise<void> {
  await apiFetch("/api/v1/platform/auth/logout", { method: "POST" });
}

export async function platformMe(): Promise<PlatformSession | null> {
  try {
    return await apiFetch<PlatformSession>("/api/v1/platform/me");
  } catch {
    return null;
  }
}

export interface TenantSession {
  email: string;
  role: string;
  tenant_id: string;
  tenant_slug?: string;
  unit_id?: string | null;
  display_name?: string | null;
  must_change_password?: boolean;
}

export interface TenantChoice {
  slug: string;
  name: string;
}

/** Erro 409: o e-mail existe em mais de uma loja e o slug é obrigatório. */
export class TenantLoginAmbiguousError extends Error {
  tenants: TenantChoice[];
  constructor(tenants: TenantChoice[]) {
    super("e-mail cadastrado em mais de uma loja; informe o slug");
    this.name = "TenantLoginAmbiguousError";
    this.tenants = tenants;
  }
}

export async function tenantLogin(email: string, password: string, slug?: string | null): Promise<TenantSession> {
  const res = await fetch(`${getApiBase()}/api/v1/auth/login`, {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email, password, slug: slug || undefined }),
  });
  if (res.status === 409) {
    const payload = (await res.json().catch(() => null)) as { tenants?: TenantChoice[] } | null;
    throw new TenantLoginAmbiguousError(payload?.tenants ?? []);
  }
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    let message = text || `Falha na API (HTTP ${res.status})`;
    try {
      const parsed = JSON.parse(text);
      if (parsed?.error) message = parsed.error;
    } catch {}
    throw new Error(message);
  }
  return res.json() as Promise<TenantSession>;
}

export async function tenantChangePassword(currentPassword: string, newPassword: string): Promise<void> {
  await apiFetch("/api/v1/auth/password", {
    method: "POST",
    body: JSON.stringify({ current_password: currentPassword, new_password: newPassword }),
  });
}

export async function tenantLogout(): Promise<void> {
  await apiFetch("/api/v1/auth/logout", { method: "POST" });
}

export async function tenantMe(): Promise<TenantSession | null> {
  try {
    return await apiFetch<TenantSession>("/api/v1/auth/me");
  } catch {
    return null;
  }
}

export const inTauri = () => false;

export async function kioskAdminStatus(): Promise<{ configured: boolean }> {
  return { configured: false };
}
export async function syncStatus(): Promise<{ online: boolean; pending: number; conflicts: number; last_error?: string | null }> {
  return apiFetch("/api/v1/sync/status");
}
export async function listPromotions(): Promise<any[]> {
  return apiFetch("/api/v1/platform/commercial/catalog");
}
export async function cloudOwnerBranding(): Promise<TenantBranding> {
  return apiFetch<TenantBranding>("/api/v1/owner/branding");
}

export async function cloudSaveOwnerBranding(displayName: string, logoDataUrl: string, primaryColor?: string | null): Promise<any> {
  return apiFetch("/api/v1/owner/branding", {
    method: "POST",
    body: JSON.stringify({ displayName, logoDataUrl: logoDataUrl || null, primaryColor: primaryColor || null }),
  });
}

export async function dashboardSummary(): Promise<any> {
  return apiFetch("/api/v1/dashboard/summary");
}

export async function salesReport(from: number, to: number, paymentMethod?: string, terminalId?: string, productQuery?: string): Promise<any> {
  return apiFetch("/api/v1/reports/sales-by-unit", {
    method: "POST",
    body: JSON.stringify({ from, to, paymentMethod, terminalId, productQuery }),
  });
}

export async function cloudOwnerOverview(from: number, to: number): Promise<any> {
  return apiFetch("/api/v1/owner/overview", {
    method: "POST",
    body: JSON.stringify({ from, to }),
  });
}

export interface PlatformTenantCreateRequest {
  name: string;
  unit_name: string;
  email?: string;
  cnpj_cpf?: string;
  phone?: string;
  plan: string;
  trial_days?: number;
  create_asaas_customer?: boolean;
  billing_mode?: "paid" | "courtesy";
  courtesy_months?: number;
  courtesy_until?: string;
  courtesy_note?: string;
}

export interface PlatformTenantCreated {
  tenant_id: string;
  unit_id: string;
  name: string;
  slug: string;
  billing_mode: string;
  courtesy_until?: string | null;
  owner_email: string;
  owner_password: string;
  owner_key: string;
  trial_ends_at: string;
  asaas_customer_id?: string | null;
}

export async function platformCreateTenant(req: PlatformTenantCreateRequest): Promise<PlatformTenantCreated> {
  return apiFetch<PlatformTenantCreated>("/api/v1/platform/tenants", {
    method: "POST",
    body: JSON.stringify(req),
  });
}

export interface CourtesyRequest {
  months?: number;
  until?: string;
  note?: string;
  revoke?: boolean;
}

export async function platformSetCourtesy(
  tenantId: string,
  req: CourtesyRequest,
): Promise<{ tenant_id: string; status: string; billing_mode: string; courtesy_until?: string | null }> {
  return apiFetch(`/api/v1/platform/tenants/${tenantId}/courtesy`, {
    method: "POST",
    body: JSON.stringify(req),
  });
}

export interface TenantSubscriptionPaymentRequest {
  amount_brl_cents: number;
  billing_type: string;
  description?: string;
  validity_days?: number;
}

export interface TenantPaymentIntentOut {
  id: string;
  external_reference: string;
  amount_brl_cents: number;
  status: string;
  qr_payload?: string | null;
  expires_at?: string | null;
  checkout_url?: string | null;
}

export async function platformCreateSubscriptionPayment(
  tenantId: string,
  req: TenantSubscriptionPaymentRequest,
): Promise<TenantPaymentIntentOut> {
  return apiFetch<TenantPaymentIntentOut>(
    `/api/v1/platform/tenants/${tenantId}/subscription-payment`,
    { method: "POST", body: JSON.stringify(req) },
  );
}

export async function platformPaymentStatus(tenantId: string): Promise<TenantPaymentIntentOut> {
  return apiFetch<TenantPaymentIntentOut>(`/api/v1/platform/tenants/${tenantId}/payment-status`);
}

export async function getDisplaySettings(): Promise<any> {
  return apiFetch("/api/v1/display/settings");
}

