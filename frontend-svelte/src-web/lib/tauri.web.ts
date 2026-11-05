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
  unit_id?: string | null;
}

export async function tenantLogin(email: string, password: string): Promise<TenantSession> {
  return apiFetch<TenantSession>("/api/v1/auth/login", {
    method: "POST",
    body: JSON.stringify({ email, password }),
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

export async function getDisplaySettings(): Promise<any> {
  return apiFetch("/api/v1/display/settings");
}

