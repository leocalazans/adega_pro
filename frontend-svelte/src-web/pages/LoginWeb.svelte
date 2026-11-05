<script lang="ts">
  import { onMount } from "svelte";
  import { LockKeyhole, ShieldCheck, Store, Mail, Globe } from "@lucide/svelte";
  import { tenantLogin, tenantMe } from "../lib/tauri.web";
  import type { UserSession } from "../../src-shared/types";

  let { onLogin, storeName = "CommerceCTRL" }: { onLogin: (session: UserSession) => void; storeName?: string } = $props();

  let email = $state("");
  let password = $state("");
  let newPassword = $state("");
  let confirmation = $state("");
  let current = $state("");
  let session = $state<UserSession | null>(null);
  let firstAccess = $state(false);
  let busy = $state(false);
  let error = $state("");

  onMount(async () => {
    const existing = await tenantMe().catch(() => null);
    if (existing) {
      onLogin({
        user_id: 0,
        employee_id: null,
        display_name: existing.email,
        username: existing.email,
        role: existing.role,
        permissions: ["*"],
      });
    }
  });

  async function submit() {
    busy = true;
    error = "";
    try {
      const result = await tenantLogin(email, password);
      onLogin({
        user_id: 0,
        employee_id: null,
        display_name: result.email,
        username: result.email,
        role: result.role,
        permissions: ["*"],
      });
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Não foi possível entrar. Verifique suas credenciais.";
    } finally {
      busy = false;
    }
  }

  async function change() {
    if (newPassword !== confirmation) {
      error = "As senhas precisam ser iguais.";
      return;
    }
    busy = true;
    error = "";
    try {
      // Primeira troca de senha via tenantLogin já autenticado
      if (session) onLogin(session);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Não foi possível alterar a senha.";
    } finally {
      busy = false;
    }
  }
</script>

<main class="access-shell">
  <aside class="store-rail" aria-hidden="true">
    <div class="brand"><span class="brand-mark">_,CTRL</span><span>CommerceCTRL</span></div>
    <div class="rail-copy">
      <p>Gestão de comércio</p>
      <h1>O balcão continua, mesmo quando a rede oscila.</h1>
    </div>
    <div class="status-card">
      <span class="pulse"></span>
      <div>
        <strong>Painel administrativo</strong>
        <small>Acesso seguro via navegador</small>
      </div>
    </div>
    <div class="web-badge">
      <Globe size={14} />
      <span>Acesso web — painel administrativo</span>
    </div>
  </aside>

  <section class="access-panel">
    <div class="panel-top">
      <div class="terminal-chip">
        <Globe size={16} />
        Painel CommerceCTRL
      </div>
      <span>v0.1.2</span>
    </div>

    <div class="store-context">
      <div class="store-seal"><Store size={24} /></div>
      <div>
        <p>Você está acessando</p>
        <h2>{storeName}</h2>
      </div>
    </div>

    {#if firstAccess}
      <form onsubmit={(event) => { event.preventDefault(); change(); }}>
        <header>
          <h3>Defina sua senha</h3>
          <p>Esta senha passa a proteger as ações do seu usuário neste terminal.</p>
        </header>
        <label>
          Nova senha
          <input type="password" minlength="8" required autocomplete="new-password" bind:value={newPassword} />
        </label>
        <label>
          Confirmar nova senha
          <input type="password" minlength="8" required autocomplete="new-password" bind:value={confirmation} />
        </label>
        {#if error}<p class="error" role="alert">{error}</p>{/if}
        <button disabled={busy}><LockKeyhole size={18} /> {busy ? "Salvando…" : "Salvar e acessar"}</button>
      </form>
    {:else}
      <form onsubmit={(event) => { event.preventDefault(); submit(); }}>
        <header>
          <h3>Acesso administrativo</h3>
          <p>Acesso ao painel de gestão da plataforma.</p>
        </header>
        <label>
          E-mail
          <div class="input-wrap"><Mail size={18} /><input type="email" autocomplete="email" required bind:value={email} /></div>
        </label>
        <label>
          Senha
          <div class="input-wrap"><LockKeyhole size={18} /><input type="password" autocomplete="current-password" required bind:value={password} /></div>
        </label>
        {#if error}<p class="error" role="alert">{error}</p>{/if}
        <button disabled={busy}>
          {busy ? "Verificando acesso…" : "Acessar painel"}
        </button>
      </form>
    {/if}

    <footer><ShieldCheck size={16} /> Acesso protegido para esta operação.</footer>
  </section>
</main>

<style>
  .access-shell {
    min-height: 100vh;
    display: grid;
    grid-template-columns: minmax(360px, 1.05fr) minmax(440px, .95fr);
    background: var(--paper);
    color: var(--ink);
    font-family: Inter, system-ui, sans-serif;
  }

  .store-rail {
    min-height: 100vh;
    padding: clamp(28px, 5vw, 68px);
    display: flex;
    flex-direction: column;
    background: linear-gradient(135deg, var(--primary-dark), var(--primary));
    color: #fff;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    font-weight: 800;
    letter-spacing: -.03em;
    font-family: "Space Grotesk", sans-serif;
  }

  .brand-mark {
    display: inline-grid;
    place-items: center;
    width: 38px;
    height: 32px;
    background: var(--accent);
    color: #fff;
    font: 900 17px/1 "Space Grotesk", monospace;
    letter-spacing: -.13em;
    border-radius: 8px;
  }

  .rail-copy {
    max-width: 530px;
    margin: auto 0 42px;
  }

  .rail-copy p {
    margin: 0;
    color: rgba(255,255,255,.7);
    font-size: 12px;
    font-weight: 750;
    letter-spacing: .05em;
    text-transform: uppercase;
  }

  .rail-copy h1 {
    margin: 14px 0 0;
    font: 650 clamp(42px, 5vw, 76px)/.94 "Space Grotesk", sans-serif;
    letter-spacing: -.075em;
    line-height: .94;
  }

  .status-card {
    display: flex;
    align-items: center;
    gap: 12px;
    width: max-content;
    max-width: 100%;
    padding: 12px 15px;
    background: rgba(255,255,255,.08);
    border: 1px solid rgba(255,255,255,.15);
    border-radius: 12px;
  }

  .status-card strong, .status-card small { display: block; }
  .status-card strong { font-size: 12px; font-weight: 700; }
  .status-card small { margin-top: 3px; color: rgba(255,255,255,.65); font-size: 11px; }

  .pulse {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 5px rgba(126, 87, 194, .25);
    animation: pulse 2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: .6; transform: scale(.9); }
  }

  .web-badge {
    margin-top: auto;
    padding: 10px 14px;
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(255,255,255,.06);
    border: 1px solid rgba(255,255,255,.1);
    border-radius: 999px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: .03em;
    color: rgba(255,255,255,.8);
    width: max-content;
  }

  .access-panel {
    align-self: center;
    justify-self: center;
    width: min(440px, calc(100vw - 48px));
    padding: 0 8px;
  }

  .panel-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 48px;
    color: var(--muted);
    font-size: 12px;
    font-weight: 700;
  }

  .terminal-chip {
    display: flex;
    align-items: center;
    gap: 7px;
    color: var(--primary);
  }

  .store-context {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 34px;
  }

  .store-seal {
    display: grid;
    place-items: center;
    width: 52px;
    height: 52px;
    background: var(--primary-wash);
    color: var(--primary);
    border-radius: 14px;
  }

  .store-context h2 {
    margin: 4px 0 0;
    font: 650 25px/1.05 "Space Grotesk", sans-serif;
    letter-spacing: -.045em;
  }

  .store-context p {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
  }

  .access-panel form {
    display: grid;
    gap: 18px;
  }

  .access-panel form header {
    margin-bottom: 8px;
  }

  .access-panel h3 {
    margin: 0;
    font: 650 32px/1 "Space Grotesk", sans-serif;
    letter-spacing: -.06em;
    color: var(--ink);
  }

  .access-panel header p {
    margin: 6px 0 0;
    color: var(--muted);
    font-size: 14px;
    line-height: 1.5;
  }

  .access-panel label {
    display: grid;
    gap: 7px;
    color: var(--muted);
    font-size: 12px;
    font-weight: 700;
  }

  .input-wrap {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 14px;
    height: 48px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--surface);
    transition: border-color .15s, box-shadow .15s;
  }

  .input-wrap:focus-within {
    border-color: var(--primary);
    box-shadow: var(--focus-ring);
  }

  .input-wrap svg {
    color: var(--muted);
    flex-shrink: 0;
  }

  .input-wrap input {
    flex: 1;
    height: 100%;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--ink);
    font: 500 15px Inter, sans-serif;
  }

  .input-wrap input::placeholder {
    color: var(--line);
  }

  .error {
    margin: 0;
    padding: 12px 14px;
    border-radius: 10px;
    background: #fef2f2;
    color: var(--danger);
    font-size: 13px;
    font-weight: 600;
    border: 1px solid #fecaca;
  }

  .access-panel button {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    height: 48px;
    padding: 0 20px;
    border: 0;
    border-radius: 10px;
    background: var(--primary);
    color: #fff;
    font: 700 15px Inter, sans-serif;
    cursor: pointer;
    transition: background .15s, transform .05s;
  }

  .access-panel button:hover:not(:disabled) {
    background: var(--primary-dark);
  }

  .access-panel button:active:not(:disabled) {
    transform: scale(.99);
  }

  .access-panel button:disabled {
    opacity: .55;
    cursor: not-allowed;
  }

  footer {
    margin-top: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
  }

  footer svg {
    color: var(--success);
  }

  @media (max-width: 900px) {
    .access-shell {
      grid-template-columns: 1fr;
    }
    .store-rail {
      min-height: auto;
      padding: clamp(24px, 8vw, 48px);
      text-align: center;
    }
    .status-card {
      margin: 0 auto;
    }
    .web-badge {
      margin: 16px auto 0;
    }
    .rail-copy {
      margin: 24px auto 32px;
    }
    .access-panel {
      padding: 0 20px 40px;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .pulse { animation: none; }
    * { transition: none !important; }
  }
</style>


