<script lang="ts">
  import { onMount } from "svelte";
  import { LockKeyhole, ShieldCheck, UserRound, Monitor } from "@lucide/svelte";
  import { loginEmployee, changeOwnPassword, cloudOwnerBranding } from "../lib/tauri.desktop";
  import type { UserSession } from "../../src-shared/types";

  let { onLogin, storeName = "CommerceCTRL" }: { onLogin: (session: UserSession) => void; storeName?: string } = $props();

  let username = $state("admin");
  let password = $state("");
  let newPassword = $state("");
  let confirmation = $state("");
  let current = $state("");
  let session = $state<UserSession | null>(null);
  let firstAccess = $state(false);
  let busy = $state(false);
  let error = $state("");
  let brandColor = $state("#0D5C3A");

  onMount(async () => {
    try {
      const branding = await cloudOwnerBranding();
      brandColor = branding.primary_color || "#0D5C3A";
      storeName = branding.display_name || storeName;
      const base = brandColor.replace("#", "");
      const r = parseInt(base.substring(0, 2), 16);
      const g = parseInt(base.substring(2, 4), 16);
      const b = parseInt(base.substring(4, 6), 16);
      const darker = `rgb(${Math.max(0, r - 20)},${Math.max(0, g - 20)},${Math.max(0, b - 20)})`;
      document.documentElement.style.setProperty("--rail-gradient", `linear-gradient(135deg, ${darker}, ${brandColor})`);
    } catch {}
  });

  async function submit() {
    busy = true;
    error = "";
    try {
      const result = await loginEmployee(username, password);
      if (result.must_change_password) {
        session = result.session;
        current = password;
        firstAccess = true;
      } else {
        onLogin(result.session);
      }
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
      await changeOwnPassword(current, newPassword);
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
      <p>Operação de caixa</p>
      <h1>O balcão continua, mesmo quando a rede oscila.</h1>
    </div>
    <div class="status-card">
      <span class="pulse"></span>
      <div>
        <strong>Terminal protegido</strong>
        <small>Dados locais e sincronização segura</small>
      </div>
    </div>
  </aside>

  <section class="access-panel">
    <div class="panel-top">
      <div class="terminal-chip">
        <Monitor size={16} />
        Terminal de venda
      </div>
      <span>v0.1.2</span>
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
          <h3>Entrar no caixa</h3>
          <p>Use as credenciais liberadas pelo administrador da loja.</p>
        </header>
        <label>
          Usuário
          <div class="input-wrap"><UserRound size={18} /><input autocomplete="username" required bind:value={username} /></div>
        </label>
        <label>
          Senha
          <div class="input-wrap"><LockKeyhole size={18} /><input type="password" autocomplete="current-password" required bind:value={password} /></div>
        </label>
        {#if error}<p class="error" role="alert">{error}</p>{/if}
        <button disabled={busy}>
          {busy ? "Validando…" : "Entrar no terminal"}
        </button>
        {#if storeName !== "CommerceCTRL"}<small class="hint">Primeiro acesso: admin / 123456</small>{/if}
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
    background: var(--rail-gradient);
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
    color: var(--primary-dark);
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
    font: 700 clamp(42px, 6vw, 78px) / .95 "Space Grotesk", sans-serif;
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
    box-shadow: 0 0 0 5px rgba(212, 246, 49, .25);
    animation: pulse 2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: .6; transform: scale(.9); }
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
    background: var(--danger-wash);
    color: var(--danger-ink);
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

