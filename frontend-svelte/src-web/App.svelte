<script lang="ts">
  import { onMount } from "svelte";
  import { Menu, Moon, Sun, PanelLeftClose, PanelLeftOpen, CircleUserRound } from "@lucide/svelte";
  import { routes, publicRoutes } from "./lib/navigation";
  import { platformLogout, platformMe, tenantLogout, tenantMe } from "./lib/tauri.web";
  import type { UserSession } from "../src-shared/types";
  import IconButton from "../src-shared/IconButton.svelte";
  import LoginWeb from "./pages/LoginWeb.svelte";
  import SuperAdmin from "./pages/SuperAdmin.svelte";
  import Onboarding from "./pages/Onboarding.svelte";
  import SaasLanding from "./pages/SaasLanding.svelte";
  import DownloadPage from "./pages/Download.svelte";
  import Setup from "./pages/Setup.svelte";
  import Privacy from "./pages/Privacy.svelte";
  import Contact from "./pages/Contact.svelte";
  import Reports from "./pages/Reports.svelte";
  import Settings from "./pages/Settings.svelte";

  let path = $state(window.location.pathname);
  let sidebarOpen = $state(false);
  let sidebarCollapsed = $state(false);
  let darkTheme = $state(false);
  let session = $state<UserSession | null>(null);

  const navigate = (href: string) => {
    history.pushState({}, "", href);
    path = href;
    sidebarOpen = false;
  };

  onMount(() => {
    const existing = tenantMe().catch(() => null);
    if (existing) {
      existing.then((s) => {
        if (s && !s.must_change_password) {
          session = {
            user_id: 0,
            employee_id: null,
            display_name: s.display_name || s.email,
            username: s.email,
            role: s.role,
            permissions: ["*"],
          };
        }
      });
    }
    sidebarCollapsed = window.localStorage.getItem("commercectrl.sidebar.collapsed") === "true";
    darkTheme = window.localStorage.getItem("commercectrl.theme") === "dark";
    document.documentElement.classList.toggle("dark", darkTheme);
    const pop = () => (path = window.location.pathname);
    window.addEventListener("popstate", pop);
    return () => { window.removeEventListener("popstate", pop); };
  });

  async function logout() {
    if (session?.role === "superadmin") {
      await platformLogout().catch(() => { });
    } else {
      await tenantLogout().catch(() => { });
    }
    session = null;
    path = "/";
    history.replaceState({}, "", "/");
  }

  let title = $derived(routes.find((route) => route.href === path)?.label ?? publicRoutes.find((r) => r.href === path)?.label ?? "CommerceCTRL");
  let visibleRoutes = $derived(routes.filter((route) => session?.permissions.includes("*") || session?.permissions.includes(route.permission)));
  function loggedIn(value: UserSession) {
    session = value;
    if (value.role === "superadmin") { navigate("/superadmin"); return; }
    const allowed = visibleRoutes.some((route) => route.href === path);
    if (!allowed) navigate(visibleRoutes[0]?.href ?? "/");
  }
</script>

{#if !session && (path === "/" || path === "/signup")}
  <SaasLanding onStart={() => navigate("/onboarding")} onLogin={() => navigate("/login")} />
{:else if path === "/onboarding"}
  <Onboarding onDone={() => navigate("/download")} />
{:else if path === "/download"}
  <DownloadPage onBack={() => navigate("/")} />
{:else if path === "/setup"}
  <Setup onBack={() => navigate("/")} />
{:else if path === "/privacidade"}
  <Privacy onBack={() => navigate("/")} />
{:else if path === "/contato"}
  <Contact onBack={() => navigate("/")} />
{:else if path === "/superadmin"}
  <SuperAdmin />
{:else if !session}
  <LoginWeb storeName="CommerceCTRL" onLogin={loggedIn} />
{:else}
  <div class="app-shell">
    <aside class:open={sidebarOpen} class:collapsed={sidebarCollapsed} class="sidebar">
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

    <section class="workspace">
      <header class="topbar">
        <IconButton onclick={() => (sidebarOpen = !sidebarOpen)} title="Abrir menu" aria-label="Abrir menu">
          {#snippet icon()}<Menu size={21} />{/snippet}
        </IconButton>
        <IconButton onclick={toggleSidebarCompact} title={sidebarCollapsed ? "Expandir menu lateral" : "Recolher menu lateral"} aria-label={sidebarCollapsed ? "Expandir menu lateral" : "Recolher menu lateral"}>
          {#snippet icon()}{#if sidebarCollapsed}<PanelLeftOpen size={20} />{:else}<PanelLeftClose size={20} />{/if}{/snippet}
        </IconButton>
        <div class="page-identity"><span class="client-monogram">CC</span><div><p class="eyebrow">CommerceCTRL</p><h1>{title}</h1></div></div>
        <div class="topbar-actions">
          <IconButton onclick={toggleTheme} title="Alternar tema">
            {#snippet icon()}{#if darkTheme}<Sun size={18} />{:else}<Moon size={18} />{/if}{/snippet}
          </IconButton>
          <IconButton onclick={logout} title="Encerrar sessão">
            {#snippet icon()}<CircleUserRound size={24} />{/snippet}
            {#snippet children()}<span><strong>{session?.display_name ?? "Usuário"}</strong><small>{session?.role ?? ""} · sair</small></span>{/snippet}
          </IconButton>
        </div>
      </header>

      <main class="workspace-inner">
        {#if path === "/"}<Reports />
        {:else if path === "/reports"}<Reports />
        {:else if path === "/promotions"}<Reports />
        {:else if path === "/display"}<Reports />
        {:else if path === "/settings"}<Settings />
        {:else}<Reports />
        {/if}
      </main>
    </section>
  </div>
{/if}




