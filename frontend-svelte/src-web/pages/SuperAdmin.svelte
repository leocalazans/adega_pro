<script lang="ts">
  import { onMount } from "svelte";
  import {
    Activity,
    BadgeDollarSign,
    Building2,
    Check,
    ChevronRight,
    CircleAlert,
    Copy,
    Download,
    ExternalLink,
    Flag,
    KeyRound,
    LogOut,
    Mail,
    MonitorSmartphone,
    PencilLine,
    Plus,
    Power,
    Receipt,
    RefreshCw,
    RotateCcw,
    Search,
    Shield,
    ShieldAlert,
    Trash2,
    TrendingUp,
    UserCog,
    UsersRound,
    X,
  } from "@lucide/svelte";

  import type { PlatformTenantCreated } from "../lib/tauri.web";

  type Tenant = {
    id: string;
    name: string;
    slug?: string | null;
    units: number;
    terminals: number;
    online_terminals: number;
    subscription_status?: string | null;
    billing_mode?: string | null;
    plan?: string | null;
    courtesy_until?: string | null;
    current_period_ends_at?: string | null;
    asaas_customer_id?: string | null;
    owner_email?: string | null;
    cnpj_cpf?: string | null;
    phone?: string | null;
    monthly_brl_cents?: number;
    mrr_brl_cents?: number;
    archived_at?: string | null;
    created_at?: string | null;
  };
  type Activation = {
    id: string;
    code: string;
    expires_at: string;
    max_uses: number;
  };
  type Metrics = {
    tenants: number;
    active_subscriptions: number;
    trials: number;
    past_due: number;
    suspended: number;
    courtesy: number;
    archived: number;
    new_30d: number;
    archived_30d: number;
    mrr_cents: number;
    arr_cents: number;
    mrr_at_risk_cents: number;
    terminals: number;
    online_terminals: number;
    gmv_30d_cents: number;
    payments_received_30d: number;
    revenue_received_30d_cents: number;
  };
  type Point = { label: string; value: number };
  type Analytics = {
    new_tenants: Point[];
    revenue: Point[];
    gmv: Point[];
    plans: { plan: string; count: number; mrr_cents: number }[];
    statuses: { status: string; count: number }[];
  };
  type QueueItem = {
    tenant_id: string;
    name: string;
    slug?: string | null;
    plan?: string | null;
    status?: string | null;
    billing_mode?: string | null;
    current_period_ends_at?: string | null;
    courtesy_until?: string | null;
    asaas_customer_id?: string | null;
    monthly_brl_cents: number;
    days_left?: number | null;
    pending_amount_cents?: number | null;
    pending_payment_id?: string | null;
  };
  type Unit = { id: string; name: string; code: string; active: boolean; terminals: number };
  type Terminal = {
    id: string;
    unit_id: string;
    unit_name: string;
    name: string;
    active: boolean;
    last_seen_at?: string | null;
    created_at: string;
  };
  type TenantUser = {
    id: string;
    email: string;
    role: string;
    active: boolean;
    unit_id?: string | null;
    display_name?: string | null;
    must_change_password: boolean;
    last_login_at?: string | null;
    created_at: string;
  };
  type Payment = {
    id: string;
    amount_brl_cents: number;
    status: string;
    provider_id?: string | null;
    external_reference: string;
    asaas_status?: string | null;
    expires_at?: string | null;
    created_at: string;
    qr_payload?: string | null;
  };
  type PaymentIntent = {
    id: string;
    amount_brl_cents: number;
    status: string;
    qr_payload?: string | null;
    expires_at?: string | null;
  };
  type PlanHistory = {
    id: number;
    from_plan?: string | null;
    to_plan: string;
    changed_by?: string | null;
    note?: string | null;
    created_at: string;
  };
  type FeatureFlag = {
    key: string;
    name: string;
    description: string;
    requires_homologation: boolean;
    active: boolean;
    enabled?: boolean;
  };
  type Lead = {
    id: string;
    store_name: string;
    contact_name?: string | null;
    email?: string | null;
    phone?: string | null;
    city?: string | null;
    plan_code?: string | null;
    modality: string;
    details?: string | null;
    status: string;
    notes?: string | null;
    created_at: string;
  };
  type AuditItem = {
    id: number;
    action: string;
    admin_email?: string | null;
    tenant_id?: string | null;
    tenant_name?: string | null;
    metadata: unknown;
    created_at: string;
  };

  type Tab = "overview" | "clients" | "billing" | "audit" | "leads" | "platform";
  type DetailTab = "resumo" | "unidades" | "terminais" | "usuarios" | "cobrancas" | "flags";

  const api = "/api/v1/platform";
  const planOptions = [
    { code: "starter", label: "Começar · R$ 100" },
    { code: "fiscal", label: "Fiscal · R$ 180" },
    { code: "premium", label: "PDV Premium · R$ 250" },
  ];
  const roleOptions = ["owner", "manager", "cashier", "stock", "finance", "viewer"];
  const leadStatuses = ["new", "contacted", "qualified", "won", "lost"];
  const leadStatusLabels: Record<string, string> = {
    new: "Novo",
    contacted: "Contatado",
    qualified: "Qualificado",
    won: "Ganho",
    lost: "Perdido",
  };
  const statusLabels: Record<string, string> = {
    trial: "Teste",
    active: "Ativo",
    past_due: "Em atraso",
    suspended: "Suspenso",
    cancelled: "Cancelado",
    sem_assinatura: "Sem assinatura",
  };

  let email = $state("");
  let password = $state("");
  let authenticated = $state(false);
  let loading = $state(false);
  let error = $state("");
  let notice = $state("");
  let tab = $state<Tab>("overview");

  let tenants = $state<Tenant[]>([]);
  let selectedId = $state("");
  let query = $state("");
  let includeArchived = $state(false);
  let detailTab = $state<DetailTab>("resumo");

  let creating = $state(false);
  let name = $state("");
  let ownerEmail = $state("");
  let unitName = $state("Matriz Centro");
  let cnpjCpf = $state("");
  let phone = $state("");
  let plan = $state("starter");
  let trialDays = $state(30);
  let createAsaas = $state(false);
  let billingMode = $state<"paid" | "courtesy">("paid");
  let courtesyMonths = $state(12);
  let courtesyNote = $state("");
  let created = $state<PlatformTenantCreated | null>(null);
  let extendMonths = $state(12);
  let activation = $state<Activation | null>(null);
  let activationDays = $state(7);

  let metrics = $state<Metrics | null>(null);
  let analytics = $state<Analytics | null>(null);
  let queue = $state<QueueItem[]>([]);
  let audit = $state<{ total: number; items: AuditItem[] }>({ total: 0, items: [] });
  let auditAction = $state("");
  let auditTenant = $state("");
  let auditOffset = $state(0);
  let leads = $state<Lead[]>([]);
  let flags = $state<FeatureFlag[]>([]);

  let units = $state<Unit[]>([]);
  let terminals = $state<Terminal[]>([]);
  let users = $state<TenantUser[]>([]);
  let payments = $state<Payment[]>([]);
  let planHistory = $state<PlanHistory[]>([]);
  let tenantFlags = $state<FeatureFlag[]>([]);

  let editName = $state("");
  let editSlug = $state("");
  let editCnpj = $state("");
  let editPhone = $state("");
  let editOwnerEmail = $state("");
  let editPlan = $state("starter");
  let planNote = $state("");
  let newUnitName = $state("");
  let tenantFlagBusy = $state("");

  let archiveTarget = $state<Tenant | null>(null);
  let archiveSlug = $state("");
  let archiveReason = $state("");
  let resetPassword = $state<{ email: string; password: string } | null>(null);
  let charge = $state<{ tenant: string; payment: PaymentIntent | null } | null>(null);
  let copied = $state("");

  let filtered = $derived(
    tenants.filter((tenant) => tenant.name.toLowerCase().includes(query.toLowerCase())),
  );
  let selected = $derived(tenants.find((tenant) => tenant.id === selectedId) ?? null);
  let activeTenants = $derived(tenants.filter((tenant) => !tenant.archived_at).length);
  let terminalsTotal = $derived(
    tenants.filter((tenant) => !tenant.archived_at).reduce((total, tenant) => total + tenant.terminals, 0),
  );
  let onlineTerminals = $derived(
    tenants
      .filter((tenant) => !tenant.archived_at)
      .reduce((total, tenant) => total + tenant.online_terminals, 0),
  );
  let maxNewTenants = $derived(
    Math.max(1, ...(analytics?.new_tenants ?? []).map((point) => point.value)),
  );
  let maxRevenue = $derived(
    Math.max(1, ...(analytics?.revenue ?? []).map((point) => point.value)),
  );

  function brl(cents: number | null | undefined): string {
    return ((cents ?? 0) / 100).toLocaleString("pt-BR", {
      style: "currency",
      currency: "BRL",
    });
  }
  function date(value?: string | null): string {
    return value ? new Date(value).toLocaleDateString("pt-BR") : "—";
  }
  function dateTime(value?: string | null): string {
    return value ? new Date(value).toLocaleString("pt-BR") : "—";
  }
  function statusLabel(value?: string | null): string {
    return value ? statusLabels[value] ?? value : "Sem assinatura";
  }

  async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
    const response = await fetch(`${api}${path}`, {
      credentials: "include",
      ...init,
      headers: { "content-type": "application/json", ...(init.headers ?? {}) },
    });
    if (!response.ok) {
      const body = await response.json().catch(() => null);
      throw new Error(body?.error ?? body?.message ?? "Não foi possível concluir a operação.");
    }
    const text = await response.text();
    return (text ? JSON.parse(text) : null) as T;
  }

  function fail(cause: unknown, fallback: string) {
    error = cause instanceof Error ? cause.message : fallback;
  }

  function flash(message: string) {
    notice = message;
    window.setTimeout(() => {
      if (notice === message) notice = "";
    }, 4000);
  }

  async function refresh() {
    if (!authenticated) return;
    loading = true;
    error = "";
    try {
      tenants = await request<Tenant[]>(`/tenants?include_archived=${includeArchived}`);
      if (!selectedId || !tenants.some((tenant) => tenant.id === selectedId))
        selectedId = tenants[0]?.id ?? "";
    } catch (cause) {
      fail(cause, "Falha ao carregar tenants.");
      authenticated = false;
    } finally {
      loading = false;
    }
  }

  async function loadOverview() {
    try {
      const [nextMetrics, nextAnalytics, nextQueue] = await Promise.all([
        request<Metrics>("/metrics"),
        request<Analytics>("/analytics?months=6"),
        request<QueueItem[]>("/billing/queue"),
      ]);
      metrics = nextMetrics;
      analytics = nextAnalytics;
      queue = nextQueue;
    } catch (cause) {
      fail(cause, "Falha ao carregar indicadores.");
    }
  }

  async function loadAudit(reset = false) {
    if (reset) auditOffset = 0;
    try {
      const params = new URLSearchParams({ limit: "100", offset: String(auditOffset) });
      if (auditTenant) params.set("tenant_id", auditTenant);
      if (auditAction.trim()) params.set("action", auditAction.trim());
      audit = await request<{ total: number; items: AuditItem[] }>(`/audit?${params.toString()}`);
    } catch (cause) {
      fail(cause, "Falha ao carregar auditoria.");
    }
  }

  async function loadLeads() {
    try {
      leads = await request<Lead[]>("/leads");
    } catch (cause) {
      fail(cause, "Falha ao carregar leads.");
    }
  }

  async function loadFlags() {
    try {
      flags = await request<FeatureFlag[]>("/flags");
    } catch (cause) {
      fail(cause, "Falha ao carregar feature flags.");
    }
  }

  async function loadTenant() {
    const tenant = selected;
    if (!tenant) return;
    try {
      const [nextUnits, nextTerminals, nextUsers, nextPayments, nextHistory, nextFlags] =
        await Promise.all([
          request<Unit[]>(`/tenants/${tenant.id}/units`),
          request<Terminal[]>(`/tenants/${tenant.id}/terminals`),
          request<TenantUser[]>(`/tenants/${tenant.id}/users`),
          request<Payment[]>(`/tenants/${tenant.id}/payments`),
          request<PlanHistory[]>(`/tenants/${tenant.id}/plan-history`),
          request<FeatureFlag[]>(`/tenants/${tenant.id}/flags`),
        ]);
      units = nextUnits;
      terminals = nextTerminals;
      users = nextUsers;
      payments = nextPayments;
      planHistory = nextHistory;
      tenantFlags = nextFlags;
      editName = tenant.name;
      editSlug = tenant.slug ?? "";
      editCnpj = tenant.cnpj_cpf ?? "";
      editPhone = tenant.phone ?? "";
      editOwnerEmail = tenant.owner_email ?? "";
      editPlan = tenant.plan ?? "starter";
      planNote = "";
    } catch (cause) {
      fail(cause, "Falha ao carregar dados do tenant.");
    }
  }

  async function selectTenant(id: string, nextTab: DetailTab = "resumo") {
    selectedId = id;
    activation = null;
    detailTab = nextTab;
    await loadTenant();
  }

  async function switchTab(next: Tab) {
    tab = next;
    if (next === "overview") await loadOverview();
    if (next === "clients") {
      await refresh();
      if (selectedId) await loadTenant();
    }
    if (next === "billing") await loadOverview();
    if (next === "audit") await loadAudit(true);
    if (next === "leads") await loadLeads();
    if (next === "platform") await loadFlags();
  }

  async function login() {
    loading = true;
    error = "";
    try {
      await request("/auth/login", {
        method: "POST",
        body: JSON.stringify({ email, password }),
      });
      password = "";
      authenticated = true;
      await Promise.all([refresh(), loadOverview()]);
      if (selectedId) await loadTenant();
    } catch (cause) {
      fail(cause, "Credenciais inválidas.");
    } finally {
      loading = false;
    }
  }

  async function logout() {
    await request("/auth/logout", { method: "POST" }).catch(() => undefined);
    authenticated = false;
    tenants = [];
    selectedId = "";
    activation = null;
  }

  async function createTenant() {
    loading = true;
    error = "";
    try {
      const payload: Record<string, unknown> = {
        name,
        unit_name: unitName,
        email: ownerEmail || undefined,
        cnpj_cpf: cnpjCpf || undefined,
        phone: phone || undefined,
        plan,
        trial_days: Number(trialDays),
        create_asaas_customer: billingMode === "paid" ? createAsaas : false,
        billing_mode: billingMode,
      };
      if (billingMode === "courtesy") {
        payload.courtesy_months = Number(courtesyMonths);
        payload.courtesy_note = courtesyNote || undefined;
      }
      const response = await request<PlatformTenantCreated>("/tenants", {
        method: "POST",
        body: JSON.stringify(payload),
      });
      created = response;
      creating = false;
      name = "";
      ownerEmail = "";
      cnpjCpf = "";
      phone = "";
      await refresh();
      selectedId = response.tenant_id;
      await loadTenant();
      await loadOverview();
      flash(`Loja ${response.name} criada.`);
    } catch (cause) {
      fail(cause, "Não foi possível criar o tenant.");
    } finally {
      loading = false;
    }
  }

  async function applyCourtesy(revoke: boolean) {
    if (!selected) return;
    loading = true;
    error = "";
    try {
      await request(`/tenants/${selected.id}/courtesy`, {
        method: "POST",
        body: JSON.stringify(
          revoke
            ? { revoke: true }
            : { months: Number(extendMonths), note: courtesyNote || undefined },
        ),
      });
      await refresh();
      await loadOverview();
      flash(revoke ? "Cortesia revogada." : "Cortesia atualizada.");
    } catch (cause) {
      fail(cause, "Não foi possível atualizar a cortesia.");
    } finally {
      loading = false;
    }
  }

  async function toggleSuspension() {
    if (!selected) return;
    const suspended = selected.subscription_status === "suspended";
    loading = true;
    error = "";
    try {
      await request(`/tenants/${selected.id}/${suspended ? "restore" : "suspend"}`, {
        method: "POST",
      });
      await refresh();
      await loadTenant();
      await loadOverview();
      flash(suspended ? "Loja restaurada." : "Loja suspensa.");
    } catch (cause) {
      fail(cause, "Não foi possível alterar a suspensão.");
    } finally {
      loading = false;
    }
  }

  async function createActivation() {
    if (!selected) return;
    loading = true;
    error = "";
    try {
      const detail = await request<{ tenant_id: string; unit_id: string }>(
        `/tenants/${selected.id}`,
      );
      if (!detail.unit_id) throw new Error("Cadastre uma unidade antes de gerar o código.");
      activation = await request<Activation>(
        `/tenants/${detail.tenant_id}/units/${detail.unit_id}/activation-codes`,
        {
          method: "POST",
          body: JSON.stringify({ valid_days: Number(activationDays), max_uses: 1 }),
        },
      );
      await refresh();
    } catch (cause) {
      fail(cause, "Não foi possível gerar o código.");
    } finally {
      loading = false;
    }
  }

  async function saveTenant() {
    if (!selected) return;
    loading = true;
    error = "";
    try {
      await request(`/tenants/${selected.id}`, {
        method: "PATCH",
        body: JSON.stringify({
          name: editName,
          slug: editSlug,
          cnpj_cpf: editCnpj,
          phone: editPhone,
          owner_email: editOwnerEmail,
        }),
      });
      await refresh();
      await loadTenant();
      flash("Dados da loja atualizados.");
    } catch (cause) {
      fail(cause, "Não foi possível salvar os dados.");
    } finally {
      loading = false;
    }
  }

  async function savePlan() {
    if (!selected) return;
    loading = true;
    error = "";
    try {
      await request(`/tenants/${selected.id}/plan`, {
        method: "POST",
        body: JSON.stringify({ plan: editPlan, note: planNote || undefined }),
      });
      await refresh();
      await loadTenant();
      await loadOverview();
      flash("Plano atualizado.");
    } catch (cause) {
      fail(cause, "Não foi possível trocar o plano.");
    } finally {
      loading = false;
    }
  }

  async function resetOwner() {
    if (!selected) return;
    loading = true;
    error = "";
    try {
      const result = await request<{ password: string }>(
        `/tenants/${selected.id}/owner-password`,
        { method: "POST", body: JSON.stringify({}) },
      );
      resetPassword = { email: selected.owner_email ?? "owner", password: result.password };
    } catch (cause) {
      fail(cause, "Não foi possível redefinir a senha.");
    } finally {
      loading = false;
    }
  }

  async function impersonate() {
    if (!selected) return;
    error = "";
    const target = window.open("about:blank", "_blank");
    try {
      await request(`/tenants/${selected.id}/impersonate`, { method: "POST" });
      if (target) {
        target.opener = null;
        target.location.replace("/");
      } else {
        window.location.href = "/";
      }
      flash("Sessão de suporte aberta em nova aba (expira em 2 horas).");
    } catch (cause) {
      target?.close();
      fail(cause, "Não foi possível abrir a sessão de suporte.");
    }
  }

  function askArchive(tenant: Tenant | null) {
    if (!tenant) return;
    archiveTarget = tenant;
    archiveSlug = "";
    archiveReason = "";
  }

  async function confirmArchive() {
    if (!archiveTarget) return;
    loading = true;
    error = "";
    try {
      await request(`/tenants/${archiveTarget.id}`, {
        method: "DELETE",
        body: JSON.stringify({
          confirm_slug: archiveSlug,
          reason: archiveReason || undefined,
        }),
      });
      flash(`${archiveTarget.name} arquivado. As vendas continuam preservadas.`);
      archiveTarget = null;
      created = null;
      await refresh();
      if (selected) await loadTenant();
      await loadOverview();
    } catch (cause) {
      fail(cause, "Não foi possível arquivar a loja.");
    } finally {
      loading = false;
    }
  }

  async function unarchive(tenant: Tenant | null) {
    if (!tenant) return;
    loading = true;
    error = "";
    try {
      await request(`/tenants/${tenant.id}/unarchive`, { method: "POST" });
      flash(`${tenant.name} restaurado. Gere um novo código de ativação para os terminais.`);
      await refresh();
      await loadTenant();
      await loadOverview();
    } catch (cause) {
      fail(cause, "Não foi possível restaurar a loja.");
    } finally {
      loading = false;
    }
  }

  async function createUnit() {
    if (!selected || newUnitName.trim().length < 2) return;
    loading = true;
    try {
      await request(`/tenants/${selected.id}/units`, {
        method: "POST",
        body: JSON.stringify({ name: newUnitName }),
      });
      newUnitName = "";
      await loadTenant();
      await refresh();
      flash("Unidade criada.");
    } catch (cause) {
      fail(cause, "Não foi possível criar a unidade.");
    } finally {
      loading = false;
    }
  }

  async function updateUnit(unit: Unit, patch: Record<string, unknown>) {
    if (!selected) return;
    try {
      await request(`/tenants/${selected.id}/units/${unit.id}`, {
        method: "PATCH",
        body: JSON.stringify(patch),
      });
      await loadTenant();
    } catch (cause) {
      fail(cause, "Não foi possível atualizar a unidade.");
    }
  }

  async function updateTerminal(terminal: Terminal, patch: Record<string, unknown>) {
    if (!selected) return;
    try {
      await request(`/tenants/${selected.id}/terminals/${terminal.id}`, {
        method: "PATCH",
        body: JSON.stringify(patch),
      });
      await loadTenant();
      await refresh();
      flash(patch.active === false ? "Terminal revogado." : "Terminal atualizado.");
    } catch (cause) {
      fail(cause, "Não foi possível atualizar o terminal.");
    }
  }

  async function updateUser(user: TenantUser, patch: Record<string, unknown>) {
    if (!selected) return;
    try {
      await request(`/tenants/${selected.id}/users/${user.id}`, {
        method: "PATCH",
        body: JSON.stringify(patch),
      });
      await loadTenant();
    } catch (cause) {
      fail(cause, "Não foi possível atualizar o usuário.");
    }
  }

  async function resetUser(user: TenantUser) {
    if (!selected) return;
    try {
      const result = await request<{ password: string }>(
        `/tenants/${selected.id}/users/${user.id}/password`,
        { method: "POST" },
      );
      resetPassword = { email: user.email, password: result.password };
    } catch (cause) {
      fail(cause, "Não foi possível redefinir a senha.");
    }
  }

  async function chargeTenant(item: QueueItem | Tenant | null) {
    if (!item) return;
    const tenantId = "tenant_id" in item ? item.tenant_id : item.id;
    const amount = item.monthly_brl_cents ?? 10000;
    error = "";
    notice = "";
    try {
      const payment = await request<PaymentIntent>(`/tenants/${tenantId}/subscription-payment`, {
        method: "POST",
        body: JSON.stringify({
          amount_brl_cents: amount || 10000,
          billing_type: "PIX",
          description: `Assinatura CommerceCTRL - ${item.name}`,
          validity_days: 30,
          due_days: 3,
        }),
      });
      charge = { tenant: item.name, payment };
      await loadTenant();
      await loadOverview();
    } catch (cause) {
      fail(cause, "Não foi possível gerar a cobrança.");
    }
  }

  async function updateLead(lead: Lead, status: string) {
    try {
      await request(`/leads/${lead.id}`, {
        method: "PATCH",
        body: JSON.stringify({ status }),
      });
      await loadLeads();
    } catch (cause) {
      fail(cause, "Não foi possível atualizar o lead.");
    }
  }

  async function togglePlatformFlag(flag: FeatureFlag) {
    try {
      await request(`/flags/${flag.key}`, {
        method: "PATCH",
        body: JSON.stringify({ active: !flag.active }),
      });
      await loadFlags();
    } catch (cause) {
      fail(cause, "Não foi possível alterar a flag.");
    }
  }

  async function toggleTenantFlag(flag: FeatureFlag) {
    if (!selected) return;
    tenantFlagBusy = flag.key;
    try {
      await request(`/tenants/${selected.id}/flags/${flag.key}`, {
        method: "PUT",
        body: JSON.stringify({ enabled: !flag.enabled }),
      });
      tenantFlags = await request<FeatureFlag[]>(`/tenants/${selected.id}/flags`);
    } catch (cause) {
      fail(cause, "Não foi possível alterar a flag da loja.");
    } finally {
      tenantFlagBusy = "";
    }
  }

  async function downloadCsv(kind: "tenants" | "payments" | "audit") {
    try {
      const response = await fetch(`${api}/export/${kind}.csv`, { credentials: "include" });
      if (!response.ok) throw new Error("Falha ao exportar CSV.");
      const blob = await response.blob();
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = `commercectrl-${kind}.csv`;
      anchor.click();
      URL.revokeObjectURL(url);
    } catch (cause) {
      fail(cause, "Falha ao exportar CSV.");
    }
  }

  async function copy(value: string, key: string) {
    try {
      await navigator.clipboard.writeText(value);
      copied = key;
      window.setTimeout(() => {
        if (copied === key) copied = "";
      }, 2000);
    } catch {
      copied = "";
    }
  }

  onMount(async () => {
    try {
      await request("/me");
      authenticated = true;
      await Promise.all([refresh(), loadOverview()]);
      if (selectedId) await loadTenant();
    } catch {
      authenticated = false;
    }
  });
</script>

{#if !authenticated}
  <main class="login-shell">
    <form
      class="login-card"
      onsubmit={(event) => {
        event.preventDefault();
        login();
      }}
    >
      <Shield size={28} />
      <p>CommerceCTRL</p>
      <h1>Administração da plataforma</h1>
      <span>Entre para provisionar lojas, terminais e códigos de ativação.</span>
      <label
        >E-mail<input type="email" bind:value={email} required autocomplete="username" /></label
      >
      <label
        >Senha<input
          type="password"
          bind:value={password}
          required
          autocomplete="current-password"
        /></label
      >
      {#if error}<div class="notice error">{error}</div>{/if}
      <button disabled={loading}>{loading ? "Entrando…" : "Entrar"}</button>
    </form>
  </main>
{:else}
  <main class="superadmin">
    <header class="page-header">
      <div>
        <p>CommerceCTRL · administração da plataforma</p>
        <h1>Clientes, cobrança e operação</h1>
      </div>
      <div class="header-actions">
        <button class="outline" onclick={() => switchTab(tab)} disabled={loading}
          ><RefreshCw size={15} /> Atualizar</button
        ><button class="outline" onclick={logout}><LogOut size={15} /> Sair</button>
      </div>
    </header>

    <nav class="tabs">
      <button class:active={tab === "overview"} onclick={() => switchTab("overview")}
        ><Activity size={15} /> Visão geral</button
      >
      <button class:active={tab === "clients"} onclick={() => switchTab("clients")}
        ><UsersRound size={15} /> Clientes</button
      >
      <button class:active={tab === "billing"} onclick={() => switchTab("billing")}
        ><Receipt size={15} /> Cobrança</button
      >
      <button class:active={tab === "audit"} onclick={() => switchTab("audit")}
        ><ShieldAlert size={15} /> Auditoria</button
      >
      <button class:active={tab === "leads"} onclick={() => switchTab("leads")}
        ><Mail size={15} /> Leads</button
      >
      <button class:active={tab === "platform"} onclick={() => switchTab("platform")}
        ><Flag size={15} /> Plataforma</button
      >
    </nav>

    {#if error}<div class="notice error">{error}</div>{/if}
    {#if notice}<div class="notice ok">{notice}</div>{/if}

    {#if tab === "overview"}
      <section class="metrics">
        <article>
          <UsersRound />
          <div><small>Lojas ativas</small><b>{metrics?.active_subscriptions ?? activeTenants}</b></div>
        </article>
        <article>
          <BadgeDollarSign />
          <div><small>MRR</small><b>{brl(metrics?.mrr_cents)}</b></div>
        </article>
        <article>
          <TrendingUp />
          <div><small>ARR</small><b>{brl(metrics?.arr_cents)}</b></div>
        </article>
        <article>
          <CircleAlert />
          <div>
            <small>Em risco</small><b>{brl(metrics?.mrr_at_risk_cents)}</b>
          </div>
        </article>
        <article>
          <Activity />
          <div>
            <small>Terminais on-line</small><b
              >{metrics?.online_terminals ?? onlineTerminals} / {metrics?.terminals ??
                terminalsTotal}</b
            >
          </div>
        </article>
        <article>
          <Receipt />
          <div>
            <small>Recebido (30d)</small><b>{brl(metrics?.revenue_received_30d_cents)}</b>
          </div>
        </article>
      </section>

      <section class="kpis">
        <div><small>Novos (30d)</small><b>{metrics?.new_30d ?? 0}</b></div>
        <div><small>Testes ativos</small><b>{metrics?.trials ?? 0}</b></div>
        <div><small>Em atraso</small><b>{metrics?.past_due ?? 0}</b></div>
        <div><small>Suspensos</small><b>{metrics?.suspended ?? 0}</b></div>
        <div><small>Cortesia</small><b>{metrics?.courtesy ?? 0}</b></div>
        <div><small>Arquivados (30d)</small><b>{metrics?.archived_30d ?? 0}</b></div>
        <div><small>GMV (30d)</small><b>{brl(metrics?.gmv_30d_cents)}</b></div>
        <div><small>Cobranças pagas (30d)</small><b>{metrics?.payments_received_30d ?? 0}</b></div>
      </section>

      <section class="split">
        <article class="panel">
          <div class="panel-head">
            <div><h2>Novos clientes</h2><span>Últimos 6 meses</span></div>
            <button class="icon" onclick={() => downloadCsv("tenants")} title="Exportar CSV"
              ><Download size={15} /></button
            >
          </div>
          <div class="chart">
            {#each analytics?.new_tenants ?? [] as point}
              <div class="bar-wrap" title={`${point.label}: ${point.value}`}>
                <span>{point.value}</span>
                <div
                  class="bar"
                  style={`height:${Math.max(3, (point.value / maxNewTenants) * 100)}%`}
                ></div>
                <small>{point.label.slice(5)}</small>
              </div>
            {/each}
          </div>
        </article>
        <article class="panel">
          <div class="panel-head">
            <div><h2>Receita recebida</h2><span>Asaas · últimos 6 meses</span></div>
            <button class="icon" onclick={() => downloadCsv("payments")} title="Exportar CSV"
              ><Download size={15} /></button
            >
          </div>
          <div class="chart">
            {#each analytics?.revenue ?? [] as point}
              <div class="bar-wrap" title={`${point.label}: ${brl(point.value)}`}>
                <span>{brl(point.value)}</span>
                <div
                  class="bar money"
                  style={`height:${Math.max(3, (point.value / maxRevenue) * 100)}%`}
                ></div>
                <small>{point.label.slice(5)}</small>
              </div>
            {/each}
          </div>
        </article>
      </section>

      <section class="split">
        <article class="panel">
          <div class="panel-head">
            <div><h2>Precisa de ação</h2><span>{queue.length} loja(s) na fila</span></div>
          </div>
          <div class="mini-list">
            {#if !queue.length}<p class="empty">Nenhuma cobrança pendente.</p>{/if}
            {#each queue.slice(0, 6) as item (item.tenant_id)}
              <button
                class="mini-row"
                onclick={() => {
                  tab = "clients";
                  selectTenant(item.tenant_id);
                }}
              >
                <span class="dot" class:risk={(item.days_left ?? 99) < 0}></span>
                <span class="grow"
                  ><strong>{item.name}</strong><small
                    >{statusLabel(item.status)} · {item.days_left ?? "—"} dia(s) · {brl(
                      item.monthly_brl_cents,
                    )}/mês</small
                  ></span
                >
                <ChevronRight size={16} />
              </button>
            {/each}
          </div>
        </article>
        <article class="panel">
          <div class="panel-head"><div><h2>Distribuição por plano</h2><span>Lojas ativas</span></div></div>
          <div class="plan-list">
            {#each analytics?.plans ?? [] as item}
              <div class="plan-row">
                <strong>{item.plan}</strong>
                <span>{item.count} loja(s)</span>
                <b>{brl(item.mrr_cents)}</b>
              </div>
            {/each}
          </div>
          <div class="status-grid">
            {#each analytics?.statuses ?? [] as item}
              <div><small>{statusLabel(item.status)}</small><b>{item.count}</b></div>
            {/each}
          </div>
        </article>
      </section>
    {:else if tab === "clients"}
      <section class="admin-grid">
        <article class="tenant-list">
          <div class="section-title">
            <div>
              <h2>Clientes</h2>
              <span>{filtered.length} cadastrados</span>
            </div>
            <button
              class="add"
              onclick={() => {
                creating = true;
                activation = null;
              }}><Plus size={15} /> Novo tenant</button
            >
          </div>
          <label class="search"
            ><Search size={16} /><input bind:value={query} placeholder="Buscar mercado"
          /></label>
          <label class="check small"
            ><input
              type="checkbox"
              bind:checked={includeArchived}
              onchange={refresh}
            /> Mostrar arquivados</label
          >
          <div class="tenant-rows">
            {#if !filtered.length}<p class="empty">Nenhum cliente provisionado.</p>{/if}
            {#each filtered as tenant (tenant.id)}
              <button
                class:selected={selectedId === tenant.id}
                class:archived={Boolean(tenant.archived_at)}
                onclick={() => selectTenant(tenant.id)}
                ><span class="tenant-icon">{tenant.name.slice(0, 1)}</span><span
                  ><strong>{tenant.name}</strong><small
                    >{tenant.units} unidade{tenant.units === 1 ? "" : "s"} · {tenant.terminals}
                    terminal{tenant.terminals === 1 ? "" : "is"}</small
                  ></span
                ><i class:risk={tenant.subscription_status === "past_due" || tenant.subscription_status === "suspended"}
                  >{tenant.archived_at ? "arquivado" : statusLabel(tenant.subscription_status)}</i
                ></button
              >
            {/each}
          </div>
        </article>

        <article class="tenant-detail">
          {#if creating}
            <div class="section-title">
              <div><p>Novo cliente</p><h2>Provisionar loja</h2></div>
              <button class="outline" onclick={() => (creating = false)}
                ><X size={15} /> Fechar</button
              >
            </div>
            <form
              class="form"
              onsubmit={(event) => {
                event.preventDefault();
                createTenant();
              }}
            >
              <label
                >Nome da loja<input
                  bind:value={name}
                  required
                  minlength="2"
                  placeholder="Mercadinho Martins"
                /></label
              ><label
                >E-mail responsável<input
                  type="email"
                  bind:value={ownerEmail}
                  placeholder="dono@mercado.com.br"
                /></label
              ><label
                >CNPJ / CPF<input bind:value={cnpjCpf} inputmode="numeric"
              /></label
              ><label>Telefone<input bind:value={phone} /></label
              ><label>Unidade inicial<input bind:value={unitName} required minlength="2" /></label
              ><label
                >Plano<select bind:value={plan}
                  >{#each planOptions as option}<option value={option.code}>{option.label}</option
                    >{/each}</select
                ></label
              ><label
                >Forma de acesso<select bind:value={billingMode}
                  ><option value="paid">Pago (Asaas)</option><option value="courtesy"
                    >Cortesia (grátis)</option
                  ></select
                ></label
              >{#if billingMode === "courtesy"}<label
                  >Cortesia por (meses)<input
                    type="number"
                    min="1"
                    max="24"
                    bind:value={courtesyMonths}
                  /></label
                ><label
                  >Motivo da cortesia<input bind:value={courtesyNote}
                /></label>{:else}<label
                  >Dias de teste<input type="number" min="0" max="365" bind:value={trialDays}
                /></label>
                <label class="check"
                  ><input type="checkbox" bind:checked={createAsaas}/> Criar cliente no Asaas para
                  cobrança de assinatura</label
                >{/if}<button class="primary" disabled={loading}>Criar tenant e unidade</button>
            </form>
          {:else if selected}
            <div class="section-title">
              <div>
                <p>Tenant selecionado</p>
                <h2>{selected.name}</h2>
              </div>
              <span class={selected.archived_at ? "pill muted" : "pill ok"}
                >{selected.archived_at ? "Arquivado" : statusLabel(selected.subscription_status)}</span
              >
            </div>

            {#if created}
              <section class="admin-section credentials">
                <h3><Shield size={17} /> Credenciais de {created.name}</h3>
                <p>Guarde agora: a senha temporária não será exibida novamente.</p>
                <div class="cred-grid">
                  <div><small>Loja (slug)</small><strong>{created.slug}</strong></div>
                  <div><small>Login do painel</small><strong>{created.owner_email}</strong></div>
                  <div><small>Senha temporária</small><strong>{created.owner_password}</strong></div>
                  <div><small>Owner key (PDV)</small><strong>{created.owner_key}</strong></div>
                </div>
                <button class="outline" onclick={() => (created = null)}>Já anotei, ocultar</button>
              </section>
            {/if}

            <div class="detail-tabs">
              {#each [["resumo", "Resumo"], ["unidades", "Unidades"], ["terminais", "Terminais"], ["usuarios", "Usuários"], ["cobrancas", "Cobranças"], ["flags", "Recursos"]] as entry}
                <button
                  class:active={detailTab === entry[0]}
                  onclick={() => {
                    detailTab = entry[0] as DetailTab;
                    loadTenant();
                  }}>{entry[1]}</button
                >
              {/each}
            </div>

            {#if detailTab === "resumo"}
              <div class="detail-stats">
                <div><small>Unidades</small><strong>{selected.units}</strong></div>
                <div><small>Terminais</small><strong>{selected.terminals}</strong><span>{selected.online_terminals} on-line</span></div>
                <div><small>Plano</small><strong>{selected.plan ?? "—"}</strong><span>{brl(selected.monthly_brl_cents)}/mês</span></div>
                <div><small>Asaas</small><strong class="truncate">{selected.asaas_customer_id ?? "—"}</strong><span>{selected.asaas_customer_id ? "vinculado" : "não vinculado"}</span></div>
              </div>

              <section class="admin-section">
                <h3><Building2 size={17} /> Dados da loja</h3>
                <div class="form grid2">
                  <label>Nome<input bind:value={editName} minlength="2" /></label>
                  <label>Slug (login)<input bind:value={editSlug} /></label>
                  <label>CNPJ / CPF<input bind:value={editCnpj} /></label>
                  <label>Telefone<input bind:value={editPhone} /></label>
                  <label>E-mail do responsável<input type="email" bind:value={editOwnerEmail} /></label>
                  <button class="primary" onclick={saveTenant} disabled={loading}>Salvar dados</button>
                </div>
              </section>

              <section class="admin-section">
                <h3><BadgeDollarSign size={17} /> Plano e assinatura</h3>
                <div class="activation-row">
                  <label
                    >Plano<select bind:value={editPlan}
                      >{#each planOptions as option}<option value={option.code}>{option.label}</option
                        >{/each}</select
                    ></label
                  ><label>Observação<input bind:value={planNote} placeholder="motivo da troca" /></label
                  ><button class="primary" onclick={savePlan} disabled={loading}>Aplicar plano</button>
                </div>
                {#if planHistory.length}
                  <div class="history">
                    {#each planHistory.slice(0, 5) as item (item.id)}
                      <div><span>{item.from_plan ?? "—"} → <strong>{item.to_plan}</strong></span><small>{dateTime(item.created_at)} · {item.changed_by ?? "sistema"}</small></div>
                    {/each}
                  </div>
                {/if}
                <div class="activation-row">
                  <button class="outline" onclick={toggleSuspension} disabled={loading}
                    ><Power size={15} /> {selected.subscription_status === "suspended"
                      ? "Restaurar acesso"
                      : "Suspender loja"}</button
                  ><button class="outline" onclick={resetOwner} disabled={loading}
                    ><KeyRound size={15} /> Redefinir senha do owner</button
                  ><button class="outline" onclick={impersonate} disabled={loading}
                    ><ExternalLink size={15} /> Abrir painel da loja</button
                  >
                </div>
              </section>

              <section class="admin-section">
                <h3><Shield size={17} /> Cortesia</h3>
                {#if selected.billing_mode === "courtesy" && selected.courtesy_until}
                  <p>
                    Acesso gratuito até
                    <strong>{date(selected.courtesy_until)}</strong>.
                  </p>
                {:else}
                  <p>Loja em modo pago ({statusLabel(selected.subscription_status)}).</p>
                {/if}
                <div class="activation-row">
                  <label
                    >Estender (meses)<input type="number" min="1" max="24" bind:value={extendMonths}
                  /></label>
                  <button class="primary" onclick={() => applyCourtesy(false)} disabled={loading}
                    >Conceder / estender</button
                  ><button class="outline" onclick={() => applyCourtesy(true)} disabled={loading}
                    >Revogar</button
                  >
                </div>
              </section>

              <section class="admin-section">
                <h3><KeyRound size={17} /> Ativar terminal</h3>
                <div class="activation-row">
                  <label
                    >Dias válidos<input type="number" min="1" max="30" bind:value={activationDays}
                  /></label>
                  <button class="primary" onclick={createActivation} disabled={loading}
                    >Gerar código</button
                  >
                </div>
                {#if activation}
                  <div class="activation">
                    <span>Copie agora no instalador</span><strong>{activation.code}</strong>
                    <small>Válido até {dateTime(activation.expires_at)} · uso único</small>
                  </div>
                {/if}
              </section>

              <section class="admin-section danger-zone">
                <h3><Trash2 size={17} /> Arquivar loja</h3>
                {#if selected.archived_at}
                  <p>Arquivada em {dateTime(selected.archived_at)}. As vendas continuam preservadas.</p>
                  <button class="outline" onclick={() => unarchive(selected)} disabled={loading}
                    ><RotateCcw size={15} /> Restaurar loja</button
                  >
                {:else}
                  <p>
                    O acesso e os terminais são bloqueados, mas vendas, produtos e histórico
                    continuam guardados. Você pode restaurar depois.
                  </p>
                  <button class="danger" onclick={() => askArchive(selected)} disabled={loading}
                    ><Trash2 size={15} /> Arquivar {selected.name}</button
                  >
                {/if}
              </section>
            {:else if detailTab === "unidades"}
              <section class="admin-section">
                <h3><Building2 size={17} /> Unidades</h3>
                <div class="activation-row">
                  <label>Nova unidade<input bind:value={newUnitName} placeholder="Filial Centro" /></label>
                  <button class="primary" onclick={createUnit} disabled={loading}>Criar unidade</button>
                </div>
                <div class="table">
                  {#each units as unit (unit.id)}
                    <div class="row">
                      <span class="grow"><strong>{unit.name}</strong><small>{unit.code} · {unit.terminals} terminal(is)</small></span>
                      <span class:pill={true} class="pill" class:muted={!unit.active}>{unit.active ? "ativa" : "inativa"}</span>
                      <button class="icon" title="Renomear" onclick={() => {
                        const next = window.prompt("Novo nome da unidade", unit.name);
                        if (next && next.trim().length >= 2) updateUnit(unit, { name: next.trim() });
                      }}><PencilLine size={15} /></button>
                      <button class="icon" title={unit.active ? "Desativar" : "Ativar"} onclick={() => updateUnit(unit, { active: !unit.active })}>
                        <Power size={15} />
                      </button>
                    </div>
                  {/each}
                  {#if !units.length}<p class="empty">Nenhuma unidade.</p>{/if}
                </div>
              </section>
            {:else if detailTab === "terminais"}
              <section class="admin-section">
                <h3><MonitorSmartphone size={17} /> Terminais</h3>
                <div class="table">
                  {#each terminals as terminal (terminal.id)}
                    <div class="row">
                      <span class="grow"><strong>{terminal.name}</strong><small>{terminal.unit_name} · visto {dateTime(terminal.last_seen_at)}</small></span>
                      <span class="pill" class:muted={!terminal.active}>{terminal.active ? "ativo" : "revogado"}</span>
                      <button class="icon" title="Renomear" onclick={() => {
                        const next = window.prompt("Novo nome do terminal", terminal.name);
                        if (next && next.trim().length >= 2) updateTerminal(terminal, { name: next.trim() });
                      }}><PencilLine size={15} /></button>
                      <button class="icon" title={terminal.active ? "Revogar" : "Reativar"} onclick={() => updateTerminal(terminal, { active: !terminal.active })}>
                        <Power size={15} />
                      </button>
                    </div>
                  {/each}
                  {#if !terminals.length}<p class="empty">Nenhum terminal ativado.</p>{/if}
                </div>
              </section>
            {:else if detailTab === "usuarios"}
              <section class="admin-section">
                <h3><UsersRound size={17} /> Usuários</h3>
                <div class="table">
                  {#each users as user (user.id)}
                    <div class="row">
                      <span class="grow"><strong>{user.display_name ?? user.email}</strong><small>{user.email} · último acesso {dateTime(user.last_login_at)}</small></span>
                      <select
                        value={user.role}
                        disabled={user.role === "owner"}
                        onchange={(event) => updateUser(user, { role: (event.target as HTMLSelectElement).value })}
                      >
                        {#each roleOptions as role}<option value={role}>{role}</option>{/each}
                      </select>
                      <span class="pill" class:muted={!user.active}>{user.active ? "ativo" : "inativo"}</span>
                      <button class="icon" title="Redefinir senha" onclick={() => resetUser(user)}><KeyRound size={15} /></button>
                      <button class="icon" title={user.active ? "Desativar" : "Ativar"} onclick={() => updateUser(user, { active: !user.active })}><Power size={15} /></button>
                    </div>
                  {/each}
                  {#if !users.length}<p class="empty">Nenhum usuário.</p>{/if}
                </div>
              </section>
            {:else if detailTab === "cobrancas"}
              <section class="admin-section">
                <h3><Receipt size={17} /> Cobranças da assinatura</h3>
                <div class="activation-row">
                  <button class="primary" onclick={() => chargeTenant(selected)} disabled={loading}
                    ><Receipt size={15} /> Gerar cobrança PIX ({brl(selected.monthly_brl_cents)})</button
                  >
                </div>
                <div class="table">
                  {#each payments as payment (payment.id)}
                    <div class="row">
                      <span class="grow"><strong>{brl(payment.amount_brl_cents)}</strong><small>{dateTime(payment.created_at)} · vence {date(payment.expires_at)}</small></span>
                      <span class="pill" class:ok={payment.status === "received" || payment.status === "confirmed"} class:muted={payment.status === "pending"}>{payment.status}</span>
                      <span class="mono">{payment.provider_id ?? "—"}</span>
                    </div>
                  {/each}
                  {#if !payments.length}<p class="empty">Nenhuma cobrança gerada.</p>{/if}
                </div>
              </section>
            {:else}
              <section class="admin-section">
                <h3><Flag size={17} /> Recursos liberados</h3>
                <p>Válido para a loja inteira. Recursos desativados na plataforma não podem ser ligados.</p>
                <div class="table">
                  {#each tenantFlags as flag (flag.key)}
                    <div class="row">
                      <span class="grow"><strong>{flag.name}</strong><small>{flag.description}</small></span>
                      {#if flag.requires_homologation}<span class="pill muted">homologação</span>{/if}
                      <button
                        class="switch"
                        class:on={flag.enabled}
                        disabled={tenantFlagBusy === flag.key}
                        onclick={() => toggleTenantFlag(flag)}
                        aria-label={`Alternar ${flag.name}`}
                      ></button>
                    </div>
                  {/each}
                </div>
              </section>
            {/if}
          {:else}
            <div class="empty-detail">Selecione ou crie um tenant.</div>
          {/if}
        </article>
      </section>
    {:else if tab === "billing"}
      <section class="panel">
        <div class="panel-head">
          <div><h2>Fila de cobrança</h2><span>Vencidas, em atraso e vencendo em 10 dias</span></div>
          <button class="outline" onclick={() => downloadCsv("payments")}
            ><Download size={15} /> Exportar cobranças</button
          >
        </div>
        <div class="table">
          {#each queue as item (item.tenant_id)}
            <div class="row">
              <span class="grow"><strong>{item.name}</strong><small>{item.slug} · {item.plan ?? "—"} · {brl(item.monthly_brl_cents)}/mês</small></span>
              <span class="pill" class:risk={(item.days_left ?? 99) < 0} class:ok={item.status === "active"}>{statusLabel(item.status)}</span>
              <span class="days" class:risk={(item.days_left ?? 99) < 0}>{item.days_left ?? "—"} dia(s)</span>
              <button
                class="outline"
                disabled={loading}
                onclick={() => {
                  if (item.pending_payment_id) {
                    tab = "clients";
                    selectTenant(item.tenant_id, "cobrancas");
                  } else {
                    chargeTenant(item);
                  }
                }}
              >
                {item.pending_payment_id ? "Ver cobrança" : "Cobrar"}
              </button>
              <button class="icon" title="Abrir cliente" onclick={() => { tab = "clients"; selectTenant(item.tenant_id); }}><ChevronRight size={16} /></button>
            </div>
          {/each}
          {#if !queue.length}<p class="empty">Nenhuma loja na fila.</p>{/if}
        </div>
      </section>
    {:else if tab === "audit"}
      <section class="panel">
        <div class="panel-head">
          <div><h2>Auditoria</h2><span>{audit.total} evento(s)</span></div>
          <button class="outline" onclick={() => downloadCsv("audit")}
            ><Download size={15} /> Exportar CSV</button
          >
        </div>
        <div class="filters">
          <label
            >Loja<select bind:value={auditTenant} onchange={() => loadAudit(true)}
              ><option value="">Todas</option>{#each tenants as tenant (tenant.id)}<option
                  value={tenant.id}>{tenant.name}</option
                >{/each}</select
            ></label
          >
          <label
            >Ação<input
              bind:value={auditAction}
              placeholder="tenant., subscription., courtesy."
              onchange={() => loadAudit(true)}
            /></label
          >
          <button class="outline" onclick={() => loadAudit(true)}>Filtrar</button>
        </div>
        <div class="table">
          {#each audit.items as item (item.id)}
            <div class="row">
              <span class="grow"><strong>{item.action}</strong><small>{item.tenant_name ?? "plataforma"} · {item.admin_email ?? "sistema"}</small></span>
              <span class="mono meta">{JSON.stringify(item.metadata)}</span>
              <span class="mono">{dateTime(item.created_at)}</span>
            </div>
          {/each}
          {#if !audit.items.length}<p class="empty">Nenhum evento no filtro.</p>{/if}
        </div>
        <div class="pager">
          <button class="outline" disabled={auditOffset === 0} onclick={() => { auditOffset = Math.max(0, auditOffset - 100); loadAudit(); }}>Anterior</button>
          <span>{auditOffset + 1}–{Math.min(auditOffset + 100, audit.total)} de {audit.total}</span>
          <button class="outline" disabled={auditOffset + 100 >= audit.total} onclick={() => { auditOffset += 100; loadAudit(); }}>Próxima</button>
        </div>
      </section>
    {:else if tab === "leads"}
      <section class="panel">
        <div class="panel-head"><div><h2>Leads comerciais</h2><span>{leads.length} contato(s)</span></div></div>
        <div class="table">
          {#each leads as lead (lead.id)}
            <div class="row">
              <span class="grow"><strong>{lead.store_name}</strong><small>{lead.contact_name ?? "—"} · {lead.email ?? "—"} · {lead.phone ?? "—"} · {lead.city ?? "—"}</small></span>
              <span class="pill muted">{lead.modality === "loan" ? "comodato" : "compra"}</span>
              <span class="mono">{lead.plan_code ?? "—"}</span>
              <select value={lead.status} onchange={(event) => updateLead(lead, (event.target as HTMLSelectElement).value)}>
                {#each leadStatuses as status}<option value={status}>{leadStatusLabels[status]}</option>{/each}
              </select>
              <span class="mono">{date(lead.created_at)}</span>
            </div>
          {/each}
          {#if !leads.length}<p class="empty">Nenhum lead recebido.</p>{/if}
        </div>
      </section>
    {:else}
      <section class="panel">
        <div class="panel-head"><div><h2>Feature flags</h2><span>Disponibilidade global por recurso</span></div></div>
        <div class="table">
          {#each flags as flag (flag.key)}
            <div class="row">
              <span class="grow"><strong>{flag.name}</strong><small>{flag.description} · <code>{flag.key}</code></small></span>
              {#if flag.requires_homologation}<span class="pill muted">homologação</span>{/if}
              <span class="pill" class:muted={!flag.active}>{flag.active ? "ativa" : "desligada"}</span>
              <button class="switch" class:on={flag.active} onclick={() => togglePlatformFlag(flag)} aria-label={`Alternar ${flag.name}`}></button>
            </div>
          {/each}
        </div>
        <div class="activation-row exports">
          <button class="outline" onclick={() => downloadCsv("tenants")}><Download size={15} /> Clientes CSV</button>
          <button class="outline" onclick={() => downloadCsv("payments")}><Download size={15} /> Cobranças CSV</button>
          <button class="outline" onclick={() => downloadCsv("audit")}><Download size={15} /> Auditoria CSV</button>
        </div>
      </section>
    {/if}
  </main>
{/if}

{#if archiveTarget}
  <div class="modal-scrim">
    <form
      class="modal"
      onsubmit={(event) => {
        event.preventDefault();
        confirmArchive();
      }}
    >
      <h2><Trash2 size={18} /> Arquivar {archiveTarget.name}</h2>
      <p>
        A loja perde acesso ao painel e os terminais são bloqueados. <strong>Vendas, produtos e
        histórico continuam preservados</strong> e você pode restaurar depois.
      </p>
      <label
        >Digite <code>{archiveTarget.slug}</code> para confirmar<input
          bind:value={archiveSlug}
          autocomplete="off"
        /></label
      >
      <label>Motivo (opcional)<input bind:value={archiveReason} placeholder="Cliente encerrou" /></label>
      <div class="modal-actions">
        <button type="button" class="outline" onclick={() => (archiveTarget = null)}>Cancelar</button>
        <button
          class="danger"
          disabled={loading || archiveSlug.trim().toLowerCase() !== (archiveTarget.slug ?? "").toLowerCase()}
          >Arquivar loja</button
        >
      </div>
    </form>
  </div>
{/if}

{#if resetPassword}
  <div class="modal-scrim">
    <div class="modal">
      <h2><KeyRound size={18} /> Senha temporária</h2>
      <p>Entregue ao usuário <strong>{resetPassword.email}</strong>. A troca é exigida no próximo login.</p>
      <div class="secret">
        <strong>{resetPassword.password}</strong>
        <button class="icon" onclick={() => copy(resetPassword?.password ?? "", "pwd")} title="Copiar">
          {#if copied === "pwd"}<Check size={16} />{:else}<Copy size={16} />{/if}
        </button>
      </div>
      <div class="modal-actions">
        <button class="outline" onclick={() => (resetPassword = null)}>Fechar</button>
      </div>
    </div>
  </div>
{/if}

{#if charge}
  <div class="modal-scrim">
    <div class="modal">
      <h2><Receipt size={18} /> Cobrança de {charge.tenant}</h2>
      {#if charge.payment}
        <p>
          {brl(charge.payment.amount_brl_cents)} · status <strong>{charge.payment.status}</strong> ·
          vence {date(charge.payment.expires_at)}
        </p>
        {#if charge.payment.qr_payload}
          <div class="secret qr"><code>{charge.payment.qr_payload}</code></div>
          <button class="outline" onclick={() => copy(charge?.payment?.qr_payload ?? "", "pix")}
            >{copied === "pix" ? "Copiado!" : "Copiar código PIX"}</button
          >
        {:else}
          <p class="empty">Cobrança criada no Asaas sem QR PIX (consulte o painel do Asaas).</p>
        {/if}
      {:else}
        <p class="empty">Nenhuma cobrança retornada.</p>
      {/if}
      <div class="modal-actions">
        <button class="outline" onclick={() => (charge = null)}>Fechar</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .login-shell,
  .superadmin {
    --bg: #101615;
    --panel: #18211e;
    --panel-2: #111916;
    --line: #2c3b35;
    --text: #e7eee8;
    --muted: #93a59b;
    --green: #93d470;
    --amber: #f4be53;
    --red: #ffd2d2;
    min-height: 100vh;
    background: var(--bg);
    color: var(--text);
    font-family: Inter, sans-serif;
  }
  .login-shell {
    display: grid;
    place-items: center;
    padding: 24px;
  }
  .login-card {
    width: min(420px, 100%);
    display: grid;
    gap: 14px;
    padding: 34px;
    border: 1px solid var(--line);
    background: var(--panel);
  }
  .login-card > :global(svg) {
    color: var(--green);
  }
  .login-card p {
    color: var(--green);
    font-weight: 800;
    margin: 0;
  }
  .login-card h1 {
    font: 700 31px/1 "Space Grotesk";
    margin: 0;
  }
  .login-card > span,
  .admin-section p {
    color: var(--muted);
    font-size: 13px;
    line-height: 1.5;
  }
  .login-card label,
  .form label,
  .activation-row label,
  .filters label {
    display: grid;
    gap: 6px;
    font-size: 12px;
    font-weight: 700;
  }
  .login-card input,
  .form input,
  .form select,
  .activation-row input,
  .filters input,
  .filters select,
  .table select {
    border: 1px solid #4a5a52;
    background: #111916;
    color: var(--text);
    padding: 10px;
    font: inherit;
    font-size: 13px;
  }
  .login-card button,
  .primary {
    border: 0;
    background: var(--green);
    color: #142516;
    padding: 11px 14px;
    font-weight: 800;
    cursor: pointer;
  }
  .superadmin {
    padding: clamp(20px, 3vw, 46px);
  }
  .page-header,
  .header-actions,
  .section-title,
  .panel-head,
  .activation-row,
  .modal-actions,
  .pager {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .page-header,
  .section-title,
  .panel-head {
    justify-content: space-between;
  }
  .page-header > div:first-child p {
    color: var(--green);
    font-size: 11px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    margin: 0 0 6px;
  }
  .page-header h1 {
    font: 700 clamp(22px, 2.4vw, 32px) "Space Grotesk";
    margin: 0;
  }
  button {
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .outline {
    border: 1px solid var(--line);
    background: transparent;
    color: var(--text);
    padding: 9px 12px;
    font-weight: 700;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
  }
  .outline:hover {
    border-color: var(--green);
  }
  .danger {
    border: 1px solid #482528;
    background: #482528;
    color: #ffd2d2;
    padding: 10px 14px;
    font-weight: 800;
    display: inline-flex;
    align-items: center;
    gap: 7px;
  }
  .tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin: 22px 0 18px;
    border-bottom: 1px solid var(--line);
  }
  .tabs button {
    border: 0;
    background: transparent;
    color: var(--muted);
    padding: 11px 14px;
    font-weight: 700;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    border-bottom: 2px solid transparent;
    font-size: 13px;
  }
  .tabs button.active {
    color: var(--text);
    border-bottom-color: var(--green);
  }
  .metrics {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(170px, 1fr));
    gap: 10px;
    margin-bottom: 14px;
  }
  .metrics article {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 15px;
    background: var(--panel);
    border: 1px solid var(--line);
  }
  .metrics article :global(svg) {
    color: var(--green);
    flex: 0 0 auto;
  }
  .metrics small,
  .detail-stats small,
  .status-grid small {
    display: block;
    color: var(--muted);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .metrics b {
    font: 700 19px "Space Grotesk";
    white-space: nowrap;
  }
  .kpis {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(128px, 1fr));
    gap: 8px;
    margin-bottom: 14px;
  }
  .kpis div {
    background: var(--panel-2);
    border: 1px solid var(--line);
    padding: 10px 12px;
    min-width: 0;
  }
  .kpis small {
    display: block;
    color: var(--muted);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .kpis b {
    font: 700 16px "Space Grotesk";
  }
  .split {
    display: grid;
    grid-template-columns: 1.1fr 0.9fr;
    gap: 12px;
    margin-bottom: 12px;
  }
  .panel {
    background: var(--panel);
    border: 1px solid var(--line);
    padding: 16px;
  }
  .panel-head h2,
  .section-title h2 {
    margin: 0;
    font: 700 16px "Space Grotesk";
  }
  .panel-head span,
  .section-title span {
    color: var(--muted);
    font-size: 12px;
  }
  .icon {
    border: 1px solid var(--line);
    background: transparent;
    color: var(--muted);
    padding: 6px;
    display: inline-grid;
    place-items: center;
  }
  .icon:hover {
    color: var(--text);
    border-color: var(--green);
  }
  .chart {
    display: flex;
    align-items: flex-end;
    gap: 8px;
    height: 170px;
    margin-top: 14px;
  }
  .bar-wrap {
    flex: 1;
    display: grid;
    grid-template-rows: auto 1fr auto;
    gap: 4px;
    align-items: end;
    height: 100%;
    text-align: center;
  }
  .bar-wrap span {
    font-size: 10px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
  }
  .bar-wrap small {
    color: var(--muted);
    font-size: 10px;
  }
  .bar {
    width: 100%;
    min-height: 3px;
    background: linear-gradient(180deg, var(--green), #365e44);
    align-self: end;
  }
  .bar.money {
    background: var(--amber);
  }
  .mini-list,
  .plan-list {
    display: grid;
    gap: 6px;
    margin-top: 12px;
  }
  .mini-row {
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--panel-2);
    border: 1px solid var(--line);
    color: var(--text);
    padding: 10px;
    text-align: left;
  }
  .mini-row:hover {
    border-color: var(--green);
  }
  .mini-row .grow,
  .row .grow {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 2px;
  }
  .mini-row strong,
  .row strong {
    font-size: 13px;
  }
  .mini-row small,
  .row small {
    color: var(--muted);
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--amber);
    flex: 0 0 auto;
  }
  .dot.risk {
    background: var(--red);
  }
  .plan-row {
    display: grid;
    grid-template-columns: 1fr auto auto;
    gap: 10px;
    align-items: center;
    padding: 8px 0;
    border-bottom: 1px solid var(--line);
    font-size: 13px;
  }
  .plan-row span {
    color: var(--muted);
  }
  .status-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(100px, 1fr));
    gap: 8px;
    margin-top: 12px;
  }
  .status-grid div {
    background: var(--panel-2);
    border: 1px solid var(--line);
    padding: 8px;
  }
  .status-grid b {
    font: 700 16px "Space Grotesk";
  }
  .admin-grid {
    display: grid;
    grid-template-columns: 340px 1fr;
    gap: 12px;
    align-items: start;
  }
  .tenant-list,
  .tenant-detail {
    background: var(--panel);
    border: 1px solid var(--line);
    padding: 16px;
  }
  .tenant-list {
    display: grid;
    gap: 10px;
  }
  .add {
    border: 0;
    background: var(--green);
    color: #142516;
    font-weight: 800;
    padding: 8px 10px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    border: 1px solid var(--line);
    background: var(--panel-2);
    padding: 0 10px;
    color: var(--muted);
  }
  .search input {
    flex: 1;
    border: 0;
    background: transparent;
    color: var(--text);
    padding: 10px 0;
    outline: none;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--muted);
  }
  .check.small {
    font-size: 11px;
  }
  .tenant-rows {
    display: grid;
    gap: 6px;
    max-height: 62vh;
    overflow: auto;
  }
  .tenant-rows button {
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--panel-2);
    border: 1px solid var(--line);
    color: var(--text);
    padding: 9px;
    text-align: left;
  }
  .tenant-rows button.selected {
    border-color: var(--green);
    background: #223329;
  }
  .tenant-rows button.archived {
    opacity: 0.6;
  }
  .tenant-rows button > span:nth-child(2) {
    flex: 1;
    min-width: 0;
    display: grid;
  }
  .tenant-rows strong {
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tenant-rows small,
  .tenant-rows i {
    color: var(--muted);
    font-size: 11px;
    font-style: normal;
  }
  .tenant-rows i.risk {
    color: var(--red);
  }
  .tenant-icon {
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    background: #223329;
    color: var(--green);
    font-weight: 800;
    flex: 0 0 auto;
  }
  .empty,
  .empty-detail {
    color: var(--muted);
    font-size: 13px;
  }
  .empty-detail {
    padding: 40px;
    text-align: center;
  }
  .detail-tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin: 14px 0;
    border-bottom: 1px solid var(--line);
  }
  .detail-tabs button {
    border: 0;
    background: transparent;
    color: var(--muted);
    padding: 8px 10px;
    font-weight: 700;
    font-size: 12px;
    border-bottom: 2px solid transparent;
  }
  .detail-tabs button.active {
    color: var(--text);
    border-bottom-color: var(--green);
  }
  .detail-stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: 8px;
    margin-bottom: 12px;
  }
  .detail-stats div {
    background: var(--panel-2);
    border: 1px solid var(--line);
    padding: 10px;
    min-width: 0;
  }
  .detail-stats strong {
    display: block;
    font: 700 16px "Space Grotesk";
  }
  .detail-stats span {
    color: var(--muted);
    font-size: 11px;
  }
  .truncate {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .admin-section {
    border-top: 1px solid var(--line);
    padding: 14px 0;
  }
  .admin-section h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 8px;
    font: 700 14px "Space Grotesk";
  }
  .admin-section h3 :global(svg) {
    color: var(--green);
  }
  .form {
    display: grid;
    gap: 10px;
  }
  .form.grid2 {
    grid-template-columns: 1fr 1fr;
  }
  .form.grid2 button {
    grid-column: 1 / -1;
    justify-self: start;
  }
  .activation-row {
    flex-wrap: wrap;
    margin-top: 8px;
  }
  .activation-row label {
    min-width: 130px;
  }
  .activation {
    margin-top: 10px;
    display: grid;
    gap: 4px;
    border: 1px dashed var(--green);
    padding: 12px;
    background: #223329;
  }
  .activation span,
  .activation small {
    color: var(--muted);
    font-size: 11px;
  }
  .activation strong {
    font: 700 20px "Space Grotesk";
    letter-spacing: 0.08em;
  }
  .credentials {
    background: #223329;
    border: 1px solid var(--green);
    padding: 14px;
    margin-bottom: 12px;
  }
  .cred-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 8px;
    margin: 10px 0;
  }
  .cred-grid small {
    display: block;
    color: var(--muted);
    font-size: 10px;
    text-transform: uppercase;
  }
  .cred-grid strong {
    font-size: 12px;
    word-break: break-all;
  }
  .pill {
    border: 1px solid var(--line);
    color: var(--muted);
    padding: 3px 8px;
    font-size: 11px;
    font-weight: 700;
    text-transform: lowercase;
    white-space: nowrap;
  }
  .pill.ok {
    border-color: #365e44;
    color: var(--green);
  }
  .pill.muted {
    opacity: 0.7;
  }
  .pill.risk {
    border-color: #482528;
    color: var(--red);
  }
  .table {
    display: grid;
    gap: 6px;
    margin-top: 10px;
  }
  .table .row {
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--panel-2);
    border: 1px solid var(--line);
    padding: 9px 10px;
    min-width: 0;
  }
  .mono {
    font-family: "JetBrains Mono", Consolas, monospace;
    font-size: 11px;
    color: var(--muted);
    white-space: nowrap;
  }
  .mono.meta {
    max-width: 240px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .days {
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
  }
  .days.risk {
    color: var(--red);
    font-weight: 700;
  }
  .history {
    display: grid;
    gap: 6px;
    margin: 10px 0;
  }
  .history div {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    font-size: 12px;
    border-bottom: 1px dashed var(--line);
    padding-bottom: 4px;
  }
  .history small {
    color: var(--muted);
  }
  .danger-zone {
    border-top: 1px solid #482528;
  }
  .danger-zone h3 :global(svg) {
    color: var(--red);
  }
  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: end;
    margin: 12px 0;
  }
  .pager {
    justify-content: space-between;
    margin-top: 12px;
    font-size: 12px;
    color: var(--muted);
  }
  .exports {
    margin-top: 14px;
    border-top: 1px solid var(--line);
    padding-top: 14px;
  }
  .switch {
    width: 42px;
    height: 22px;
    border-radius: 999px;
    border: 1px solid var(--line);
    background: #111916;
    position: relative;
    flex: 0 0 auto;
  }
  .switch::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--muted);
    transition: transform 0.15s ease;
  }
  .switch.on {
    background: #26472f;
    border-color: var(--green);
  }
  .switch.on::after {
    transform: translateX(20px);
    background: var(--green);
  }
  .notice {
    padding: 10px 12px;
    border: 1px solid var(--line);
    font-size: 13px;
    margin-bottom: 12px;
  }
  .notice.error {
    border-color: #482528;
    background: #482528;
    color: #ffd2d2;
  }
  .notice.ok {
    border-color: #365e44;
    background: #26472f;
    color: var(--green);
  }
  .modal-scrim {
    position: fixed;
    inset: 0;
    background: rgba(5, 9, 7, 0.72);
    display: grid;
    place-items: center;
    padding: 20px;
    z-index: 40;
  }
  .modal {
    width: min(480px, 100%);
    background: var(--panel);
    border: 1px solid var(--line);
    padding: 20px;
    display: grid;
    gap: 12px;
  }
  .modal h2 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    font: 700 18px "Space Grotesk";
  }
  .modal p {
    margin: 0;
    color: var(--muted);
    font-size: 13px;
    line-height: 1.55;
  }
  .modal label {
    display: grid;
    gap: 6px;
    font-size: 12px;
    font-weight: 700;
    color: var(--muted);
  }
  .modal input {
    border: 1px solid #4a5a52;
    background: #111916;
    color: var(--text);
    padding: 10px;
  }
  .modal code,
  .secret code {
    color: var(--green);
    font-family: "JetBrains Mono", Consolas, monospace;
  }
  .modal-actions {
    justify-content: flex-end;
    margin-top: 4px;
  }
  .secret {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    border: 1px dashed var(--green);
    background: #223329;
    padding: 12px;
    word-break: break-all;
  }
  .secret strong {
    font: 700 18px "Space Grotesk";
    letter-spacing: 0.06em;
  }
  .secret.qr code {
    font-size: 11px;
    line-height: 1.4;
  }
  @media (max-width: 980px) {
    .admin-grid {
      grid-template-columns: 1fr;
    }
    .split {
      grid-template-columns: 1fr;
    }
    .form.grid2 {
      grid-template-columns: 1fr;
    }
  }
</style>
