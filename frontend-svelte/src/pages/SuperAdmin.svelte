<script lang="ts">
  import { onMount } from "svelte";
  import { Activity, BadgeDollarSign, Building2, CircleAlert, KeyRound, LogOut, Plus, RefreshCw, Search, Shield, UsersRound, X } from "@lucide/svelte";

  type Tenant = { id: string; name: string; units: number; terminals: number; online_terminals: number };
  type Activation = { id: string; code: string; expires_at: string; max_uses: number };
  const api = "/api/v1/platform";
  let email = $state("");
  let password = $state("");
  let authenticated = $state(false);
  let loading = $state(false);
  let error = $state("");
  let tenants = $state<Tenant[]>([]);
  let selectedId = $state("");
  let query = $state("");
  let creating = $state(false);
  let name = $state("");
  let unitName = $state("Matriz Centro");
  let plan = $state("starter");
  let trialDays = $state(30);
  let activation = $state<Activation | null>(null);
  let activationDays = $state(7);

  let filtered = $derived(tenants.filter((tenant) => tenant.name.toLowerCase().includes(query.toLowerCase())));
  let selected = $derived(tenants.find((tenant) => tenant.id === selectedId) ?? null);
  let activeTenants = $derived(tenants.length);
  let terminals = $derived(tenants.reduce((total, tenant) => total + tenant.terminals, 0));
  let onlineTerminals = $derived(tenants.reduce((total, tenant) => total + tenant.online_terminals, 0));

  async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
    const response = await fetch(`${api}${path}`, {
      credentials: "include",
      ...init,
      headers: { "content-type": "application/json", ...(init.headers ?? {}) }
    });
    if (!response.ok) {
      const body = await response.json().catch(() => null);
      throw new Error(body?.error ?? body?.message ?? "Não foi possível concluir a operação.");
    }
    return response.json() as Promise<T>;
  }

  async function refresh() {
    if (!authenticated) return;
    loading = true;
    error = "";
    try {
      tenants = await request<Tenant[]>("/tenants");
      if (!selectedId || !tenants.some((tenant) => tenant.id === selectedId)) selectedId = tenants[0]?.id ?? "";
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Falha ao carregar tenants.";
      authenticated = false;
    } finally {
      loading = false;
    }
  }

  async function login() {
    loading = true;
    error = "";
    try {
      await request("/auth/login", { method: "POST", body: JSON.stringify({ email, password }) });
      password = "";
      authenticated = true;
      await refresh();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Credenciais inválidas.";
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
      const created = await request<{ tenant_id: string }>("/tenants", {
        method: "POST",
        body: JSON.stringify({ name, unit_name: unitName, plan, trial_days: Number(trialDays) })
      });
      creating = false;
      name = "";
      await refresh();
      selectedId = created.tenant_id;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Não foi possível criar o tenant.";
    } finally {
      loading = false;
    }
  }

  async function createActivation() {
    if (!selected) return;
    loading = true;
    error = "";
    try {
      const detail = await request<{ tenant_id: string; unit_id: string }>(`/tenants/${selected.id}`, { method: "GET" });
      activation = await request<Activation>(`/tenants/${detail.tenant_id}/units/${detail.unit_id}/activation-codes`, {
        method: "POST",
        body: JSON.stringify({ valid_days: Number(activationDays), max_uses: 1 })
      });
      await refresh();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Não foi possível gerar o código.";
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    try {
      await request("/me");
      authenticated = true;
      await refresh();
    } catch { authenticated = false; }
  });
</script>

{#if !authenticated}
  <main class="login-shell">
    <form class="login-card" onsubmit={(event) => { event.preventDefault(); login(); }}>
      <Shield size={28} />
      <p>CommerceCTRL</p>
      <h1>Administração da plataforma</h1>
      <span>Entre para provisionar lojas, terminais e códigos de ativação.</span>
      <label>E-mail<input type="email" bind:value={email} required autocomplete="username" /></label>
      <label>Senha<input type="password" bind:value={password} required autocomplete="current-password" /></label>
      {#if error}<div class="notice error">{error}</div>{/if}
      <button disabled={loading}>{loading ? "Entrando…" : "Entrar"}</button>
    </form>
  </main>
{:else}
  <main class="superadmin">
    <header class="page-header"><div><p>CommerceCTRL · administração da plataforma</p><h1>Clientes e operação</h1></div><div class="header-actions"><button class="outline" onclick={refresh} disabled={loading}><RefreshCw size={15} /> Atualizar</button><button class="outline" onclick={logout}><LogOut size={15} /> Sair</button></div></header>
    {#if error}<div class="notice error">{error}</div>{/if}
    <section class="metrics"><article><UsersRound/><div><small>Tenants</small><b>{activeTenants}</b></div></article><article><BadgeDollarSign/><div><small>Plano inicial</small><b>R$ 100</b></div></article><article><Activity/><div><small>Terminais on-line</small><b>{onlineTerminals} / {terminals}</b></div></article><article><CircleAlert/><div><small>Ação necessária</small><b>{activation ? "1" : "0"}</b></div></article></section>
    <section class="admin-grid"><article class="tenant-list"><div class="section-title"><div><h2>Clientes</h2><span>{filtered.length} cadastrados</span></div><button class="add" onclick={() => { creating = true; activation = null; }}><Plus size={15} /> Novo tenant</button></div><label class="search"><Search size={16}/><input bind:value={query} placeholder="Buscar mercado"/></label><div class="tenant-rows">{#if !filtered.length}<p class="empty">Nenhum cliente provisionado.</p>{/if}{#each filtered as tenant}<button class:selected={selectedId===tenant.id} onclick={() => { selectedId = tenant.id; activation = null; }}><span class="tenant-icon">{tenant.name.slice(0,1)}</span><span><strong>{tenant.name}</strong><small>{tenant.units} unidade{tenant.units === 1 ? "" : "s"} · {tenant.terminals} terminal{tenant.terminals === 1 ? "" : "is"}</small></span><i>{tenant.online_terminals} on-line</i></button>{/each}</div></article>
      <article class="tenant-detail">{#if creating}<div class="section-title"><div><p>Novo cliente</p><h2>Provisionar loja</h2></div><button class="outline" onclick={() => creating = false}><X size={15}/> Fechar</button></div><form class="form" onsubmit={(event) => { event.preventDefault(); createTenant(); }}><label>Nome da loja<input bind:value={name} required minlength="2" placeholder="Mercadinho Martins"/></label><label>Unidade inicial<input bind:value={unitName} required minlength="2"/></label><label>Plano<select bind:value={plan}><option value="starter">Começar · R$ 100</option><option value="fiscal">Fiscal · R$ 180</option><option value="premium">PDV Premium · R$ 250</option></select></label><label>Dias de teste<input type="number" min="0" max="365" bind:value={trialDays}/></label><button class="primary" disabled={loading}>Criar tenant e unidade</button></form>{:else if selected}<div class="section-title"><div><p>Tenant selecionado</p><h2>{selected.name}</h2></div><span class="ok">Operacional</span></div><div class="detail-stats"><div><small>Unidades</small><strong>{selected.units}</strong><span>Matriz e filiais</span></div><div><small>Terminais</small><strong>{selected.terminals}</strong><span>{selected.online_terminals} on-line</span></div><div><small>Ativação</small><strong>Uso único</strong><span>Vinculada à unidade</span></div></div><section class="admin-section"><h3><KeyRound size={17}/> Ativar terminal</h3><p>Gere um código para instalar o PDV neste caixa. O código só é exibido uma vez.</p><div class="activation-row"><label>Dias válidos<input type="number" min="1" max="30" bind:value={activationDays}/></label><button class="primary" onclick={createActivation} disabled={loading}>Gerar código</button></div>{#if activation}<div class="activation"><span>Copie agora no instalador</span><strong>{activation.code}</strong><small>Válido até {new Date(activation.expires_at).toLocaleString("pt-BR")} · uso único</small></div>{/if}</section>{:else}<div class="empty-detail">Selecione ou crie um tenant.</div>{/if}</article></section>
  </main>
{/if}

<style>
  .login-shell,.superadmin{--bg:#101615;--panel:#18211e;--line:#2c3b35;--text:#e7eee8;--muted:#93a59b;--green:#93d470;--amber:#f4be53;min-height:100vh;background:var(--bg);color:var(--text);font-family:Inter,sans-serif}.login-shell{display:grid;place-items:center;padding:24px}.login-card{width:min(420px,100%);display:grid;gap:14px;padding:34px;border:1px solid var(--line);background:var(--panel)}.login-card>svg{color:var(--green)}.login-card p{color:var(--green);font-weight:800;margin:0}.login-card h1{font:700 31px/1 "Space Grotesk";margin:0}.login-card>span,.admin-section p{color:var(--muted);font-size:13px;line-height:1.5}.login-card label,.form label,.activation-row label{display:grid;gap:6px;font-size:12px;font-weight:700}.login-card input,.form input,.form select,.activation-row input{border:1px solid #4a5a52;background:#111916;color:var(--text);padding:11px}.login-card button,.primary{border:0;background:var(--green);color:#142516;padding:12px;font-weight:800}.superadmin{padding:clamp(24px,4vw,58px)}.page-header,.header-actions,.section-title,.activation-row{display:flex;justify-content:space-between;align-items:center;gap:12px}.page-header{margin-bottom:30px}.page-header p,.section-title p{font-size:12px;color:var(--muted);margin:0 0 6px}.page-header h1{font:700 clamp(34px,4vw,55px)/1 "Space Grotesk";letter-spacing:-.06em;margin:0}.outline{display:flex;align-items:center;gap:6px;border:1px solid #64766b;background:transparent;color:var(--text);padding:9px 11px;font-size:12px}.metrics{display:grid;grid-template-columns:repeat(4,1fr);gap:12px;margin-bottom:18px}.metrics article{padding:19px;background:var(--panel);border:1px solid var(--line);display:flex;gap:14px;align-items:center}.metrics :global(svg){color:var(--green)}.metrics small,.detail-stats small{display:block;color:var(--muted);font-size:11px}.metrics b{font:700 25px "Space Grotesk"}.admin-grid{display:grid;grid-template-columns:minmax(270px,.73fr) minmax(0,1.6fr);gap:18px}.tenant-list,.tenant-detail{background:var(--panel);border:1px solid var(--line)}.section-title{padding:22px;border-bottom:1px solid var(--line)}.section-title h2{font:700 21px "Space Grotesk";margin:0}.section-title span{font-size:12px;color:var(--muted)}button{font:inherit;cursor:pointer}.add{display:flex;align-items:center;gap:5px;background:var(--green);border:0;padding:8px 10px;font-weight:700;color:#142516}.search{margin:18px;display:flex;align-items:center;gap:8px;border-bottom:1px solid #4b5b53;padding-bottom:9px;color:var(--muted)}.search input{border:0;outline:0;background:transparent;color:var(--text);width:100%}.tenant-rows{display:grid}.tenant-rows button{padding:15px 18px;display:grid;grid-template-columns:34px 1fr auto;align-items:center;gap:10px;text-align:left;color:var(--text);background:transparent;border:0;border-top:1px solid var(--line)}.tenant-rows button.selected{background:#223329}.tenant-icon{display:grid;place-items:center;width:30px;height:30px;background:#365e44;color:#dff3d3;font-weight:800}.tenant-rows strong,.tenant-rows small{display:block}.tenant-rows small{font-size:11px;color:var(--muted);margin-top:3px}.tenant-rows i{font-size:10px;color:var(--green);font-style:normal}.detail-stats{display:grid;grid-template-columns:repeat(3,1fr);padding:22px;gap:12px}.detail-stats div{border-left:2px solid var(--green);padding-left:10px}.detail-stats strong,.detail-stats span{display:block}.detail-stats strong{margin:5px 0}.detail-stats span{font-size:11px;color:var(--muted)}.admin-section{border-top:1px solid var(--line);padding:22px}.admin-section h3{display:flex;gap:8px;align-items:center;font:650 15px "Space Grotesk";margin:0 0 8px}.admin-section h3 :global(svg){color:var(--green)}.activation-row{margin-top:18px;justify-content:flex-start}.activation-row input{width:90px}.activation{display:grid;gap:5px;margin-top:18px;padding:16px;border:1px solid #6b8b57;background:#223329}.activation span,.activation small{font-size:11px;color:#c5d9bb}.activation strong{font:700 24px "Space Grotesk";letter-spacing:.04em;color:#fff}.form{display:grid;grid-template-columns:1fr 1fr;gap:14px;padding:22px}.form .primary{grid-column:1/-1}.empty,.empty-detail{padding:28px;color:var(--muted);font-size:13px}.notice{margin:0 0 18px;padding:12px;font-size:13px}.notice.error{background:#482528;color:#ffd2d2}.ok{color:#bdeaa7!important;background:#26472f;padding:4px 7px}.primary:disabled,.login-card button:disabled,.outline:disabled{opacity:.55;cursor:wait}@media(max-width:900px){.metrics{grid-template-columns:repeat(2,1fr)}.admin-grid{grid-template-columns:1fr}.tenant-list{max-height:370px;overflow:auto}}@media(max-width:600px){.superadmin{padding:20px}.page-header{align-items:start;flex-direction:column}.metrics{grid-template-columns:1fr 1fr}.detail-stats,.form{grid-template-columns:1fr}.header-actions{width:100%}.header-actions button{flex:1;justify-content:center}}
</style>
