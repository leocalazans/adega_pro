<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { Menu, Moon, Sun, PanelLeftClose, PanelLeftOpen, CircleUserRound, Wifi, WifiOff, LockKeyhole } from "@lucide/svelte";
  import { routes } from "./lib/navigation";
  import { cloudOwnerBranding, configureKioskAdminPin, currentSession, exitKioskAsAdmin, inTauri, kioskAdminStatus, logoutEmployee, syncStatus } from "./lib/tauri";
  import type { UserSession } from "./lib/types";
  import Dashboard from "./pages/Dashboard.svelte";
  import Pos from "./pages/Pos.svelte";
  import Products from "./pages/Products.svelte";
  import Restock from "./pages/Restock.svelte";
  import CashClosing from "./pages/CashClosing.svelte";
  import Display from "./pages/Display.svelte";
  import Expenses from "./pages/Expenses.svelte";
  import Employees from "./pages/Employees.svelte";
  import Suppliers from "./pages/Suppliers.svelte";
  import Contacts from "./pages/Contacts.svelte";
  import Reports from "./pages/Reports.svelte";
  import Promotions from "./pages/Promotions.svelte";
  import Settings from "./pages/Settings.svelte";
  import TimeClock from "./pages/TimeClock.svelte";
  import Labels from "./pages/Labels.svelte";
  import Login from "./pages/Login.svelte";
  import Accounting from "./pages/Accounting.svelte";
  import SaasLanding from "./pages/SaasLanding.svelte";
  import Onboarding from "./pages/Onboarding.svelte";
  import SuperAdmin from "./pages/SuperAdmin.svelte";
  import DownloadPage from "./pages/Download.svelte";
  import Setup from "./pages/Setup.svelte";
  import Privacy from "./pages/Privacy.svelte";

  let path = $state(window.location.pathname);
  let sidebarOpen = $state(false);
  let sidebarCollapsed = $state(false);
  let darkTheme = $state(false);
  let online = $state(false);
  let pending = $state(0);
  let conflicts = $state(0);
  let syncError = $state("");
  let isDesktop = $state(inTauri());
  let adminExitOpen = $state(false);
  let adminConfigured = $state(true);
  let adminPin = $state("");
  let adminPinConfirmation = $state("");
  let adminExitError = $state("");
  let adminExitBusy = $state(false);
  let clientName = $state("Sua loja");
  let clientLogo = $state("");
  let session = $state<UserSession|null>(null);

  const navigate = (href: string) => {
    history.pushState({}, "", href);
    path = href;
    sidebarOpen = false;
  };

  onMount(() => {
    isDesktop = inTauri();
    if(isDesktop) currentSession().then(value=>session=value).catch(()=>session=null);
    sidebarCollapsed = window.localStorage.getItem("commercectrl.sidebar.collapsed") === "true";
    darkTheme = window.localStorage.getItem("commercectrl.theme") === "dark";
    document.documentElement.classList.toggle("dark", darkTheme);
    const applyBranding = (branding: { name?: string; logo?: string }) => { clientName = branding.name || "Sua loja"; clientLogo = branding.logo || ""; };
    try { const saved = JSON.parse(localStorage.getItem("commercectrl.client.branding") ?? "null"); if (saved) applyBranding(saved); } catch {}
    const brandingChanged = (event: Event) => applyBranding((event as CustomEvent<{ name: string; logo: string }>).detail);
    window.addEventListener("commercectrl-branding", brandingChanged);
    if (isDesktop) cloudOwnerBranding().then((saved)=>{const branding={name:saved.display_name,logo:saved.logo_data_url||""};localStorage.setItem("commercectrl.client.branding",JSON.stringify(branding));applyBranding(branding);window.dispatchEvent(new CustomEvent("commercectrl-branding",{detail:branding}))}).catch(()=>{});
    const pop = () => (path = window.location.pathname);
    window.addEventListener("popstate", pop);
    const poll = async () => {
      const status = await syncStatus().catch(() => ({ online: false, pending: 0, last_sync_ms: 0, conflicts: 0, last_error: null }));
      online = status.online;
      pending = status.pending;
      conflicts = status.conflicts;
      syncError = status.last_error ?? "";
    };
    poll();
    const timer = window.setInterval(poll, 5000);
    const shortcut = (event: KeyboardEvent) => {
      if (event.ctrlKey && event.shiftKey && event.key.toLowerCase() === "q") {
        event.preventDefault();
        openAdminExit();
      }
    };
    window.addEventListener("keydown", shortcut);
    let unlistenKioskExit: (() => void) | undefined;
    if (isDesktop) {
      listen("kiosk-exit-requested", () => openAdminExit()).then((unlisten) => unlistenKioskExit = unlisten);
    }
    return () => { window.removeEventListener("popstate", pop); window.removeEventListener("keydown", shortcut); window.removeEventListener("commercectrl-branding", brandingChanged); unlistenKioskExit?.(); clearInterval(timer); };
  });

  function toggleSidebarCompact() {
    sidebarCollapsed = !sidebarCollapsed;
    window.localStorage.setItem("commercectrl.sidebar.collapsed", String(sidebarCollapsed));
  }
  function toggleTheme(){ darkTheme=!darkTheme; window.localStorage.setItem("commercectrl.theme",darkTheme?"dark":"light"); document.documentElement.classList.toggle("dark",darkTheme); window.dispatchEvent(new CustomEvent("commercectrl-theme",{detail:darkTheme})); }

  async function openAdminExit() {
    if (!inTauri()) return;
    adminExitError = "";
    adminPin = "";
    adminPinConfirmation = "";
    adminConfigured = (await kioskAdminStatus()).configured;
    adminExitOpen = true;
  }

  async function confirmAdminExit() {
    adminExitError = "";
    if (!adminConfigured && adminPin !== adminPinConfirmation) {
      adminExitError = "As senhas não coincidem.";
      return;
    }
    adminExitBusy = true;
    try {
      if (!adminConfigured) await configureKioskAdminPin(adminPin);
      await exitKioskAsAdmin(adminPin);
    } catch (cause) {
      adminExitError = cause instanceof Error ? cause.message : "Não foi possível validar o administrador.";
    } finally {
      adminExitBusy = false;
    }
  }

  let title = $derived(routes.find((route) => route.href === path)?.label ?? "CommerceCTRL");
  let isPosRoute = $derived(path === "/pos" || path.startsWith("/pos/"));
  let visibleRoutes = $derived(routes.filter(route=>session?.permissions.includes("*")||session?.permissions.includes(route.permission)));
  function loggedIn(value:UserSession){session=value;const allowed=visibleRoutes.some(route=>route.href===path);if(!allowed)navigate(visibleRoutes[0]?.href??"/time-clock")}
  async function logout(){await logoutEmployee();session=null;path="/";history.replaceState({},"","/")}
</script>

{#if !isDesktop && !session && (path === "/" || path === "/signup")}
  <SaasLanding onStart={() => navigate("/onboarding")} onLogin={() => navigate("/login")} />
{:else if path === "/onboarding"}
  <Onboarding onDone={() => navigate("/download")} />
{:else if !isDesktop && path === "/download"}
  <DownloadPage onBack={() => navigate("/")} />
{:else if !isDesktop && path === "/setup"}
  <Setup onBack={() => navigate("/")} />
{:else if !isDesktop && path === "/privacidade"}
  <Privacy onBack={() => navigate("/")} />
{:else if path === "/display"}
  <Display />
{:else if !session}
  <Login storeName={isDesktop ? clientName : "CommerceCTRL"} onLogin={loggedIn}/>
{:else}
  <div class="app-shell">
    <aside class:open={sidebarOpen} class:collapsed={sidebarCollapsed || isPosRoute} class:pos-dock={isPosRoute} class="sidebar">
      <button class="brand" onclick={() => navigate("/")} aria-label="Ir ao dashboard">
        <span class="brand-mark system-wordmark">_,CTRL</span>
        <span>CommerceCTRL</span>
      </button>
      <nav aria-label="Navegação principal">
        {#each visibleRoutes as route}
          <button class:active={path === route.href} onclick={() => navigate(route.href)} title={route.label}>
            <route.icon size={19} />
            <span>{route.label}</span>
          </button>
        {/each}
      </nav>
    </aside>
    <button class:visible={sidebarOpen} class="scrim" onclick={() => (sidebarOpen = false)} aria-label="Fechar menu"></button>

    <section class:pos-mode={isPosRoute} class="workspace">
      {#if !isPosRoute}<header class="topbar">
        <button class="icon-button mobile-menu" onclick={() => (sidebarOpen = !sidebarOpen)} aria-label="Abrir menu"><Menu size={21} /></button>
        <button class="icon-button desktop-menu-toggle" onclick={toggleSidebarCompact} aria-label={sidebarCollapsed ? "Expandir menu lateral" : "Recolher menu lateral"} title={sidebarCollapsed ? "Expandir menu" : "Recolher menu"}>{#if sidebarCollapsed}<PanelLeftOpen size={20}/>{:else}<PanelLeftClose size={20}/>{/if}</button>
        <div class="page-identity">{#if clientLogo}<img src={clientLogo} alt={`Logo ${clientName}`}/>{:else}<span class="client-monogram">{clientName.split(" ").map((word)=>word[0]).join("").slice(0,2)}</span>{/if}<div><p class="eyebrow">{clientName}</p><h1>{title}</h1></div></div>
        <div class="topbar-actions"><button class="icon-button" onclick={toggleTheme} title="Alternar tema">{#if darkTheme}<Sun size={18}/>{:else}<Moon size={18}/>{/if}</button>
          <span class:online class="connection" class:warning={conflicts>0} title={syncError}>{#if online}<Wifi size={15} /> Online{:else}<WifiOff size={15} /> Offline{/if}{#if pending} · {pending} pendentes{/if}{#if conflicts} · {conflicts} conflitos{/if}</span>
          <button class="profile" onclick={logout} title="Encerrar sessão"><CircleUserRound size={24}/><span><strong>{session.display_name}</strong><small>{session.role} · sair</small></span></button>
          {#if isDesktop&&session.permissions.includes("*")}<button class="icon-button" onclick={openAdminExit} title="Fechar aplicativo (Ctrl+Shift+Q)"><LockKeyhole size={19}/></button>{/if}
        </div>
      </header>{/if}

      <main class:pos-main={isPosRoute}>
        {#if path === "/"}<Dashboard />
        {:else if path === "/pos"}<Pos />
        {:else if path === "/products"}<Products />
        {:else if path === "/restock"}<Restock />
        {:else if path === "/cash-closing"}<CashClosing />
        {:else if path === "/expenses"}<Expenses />
        {:else if path === "/accounting"}<Accounting />
        {:else if path === "/employees"}<Employees />
        {:else if path === "/time-clock"}<TimeClock />
        {:else if path === "/labels"}<Labels />
        {:else if path === "/reports"}<Reports />
        {:else if path === "/suppliers"}<Suppliers />
        {:else if path === "/contacts"}<Contacts />
        {:else if path === "/promotions"}<Promotions />
        {:else if path === "/settings"}<Settings />
        {:else if path === "/superadmin" && session.username === "superadmin@commercecontrol.com.br"}<SuperAdmin />
        {:else}<Dashboard />{/if}
      </main>
    </section>
  </div>
{/if}

{#if adminExitOpen}
  <div class="modal-backdrop" role="presentation">
    <form class="modal card" onsubmit={(event) => { event.preventDefault(); confirmAdminExit(); }}>
      <div class="card-header"><div><h2>{adminConfigured ? "Encerrar modo quiosque" : "Definir senha administrativa"}</h2><p class="small muted">Somente o administrador pode encerrar o aplicativo.</p></div></div>
      <div class="card-body form-grid">
        {#if !adminConfigured}<p class="wide small warning-text">Primeira configuração: guarde esta senha. Ela será exigida para fechar o PDV.</p>{/if}
        <label class="wide">Senha administrativa<input class="input" type="password" minlength="6" maxlength="128" required autocomplete="current-password" bind:value={adminPin}/></label>
        {#if !adminConfigured}<label class="wide">Confirmar senha<input class="input" type="password" minlength="6" maxlength="128" required autocomplete="new-password" bind:value={adminPinConfirmation}/></label>{/if}
        {#if adminExitError}<p class="wide error-text" role="alert">{adminExitError}</p>{/if}
      </div>
      <div class="card-footer modal-actions"><button class="button secondary" type="button" onclick={() => adminExitOpen = false} disabled={adminExitBusy}>Cancelar</button><button class="button" disabled={adminExitBusy}>{adminExitBusy ? "Validando…" : adminConfigured ? "Validar e fechar" : "Salvar e fechar"}</button></div>
    </form>
  </div>
{/if}
