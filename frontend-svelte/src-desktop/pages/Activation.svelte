<script lang="ts">
  import { onMount } from "svelte";
  import { claimActivation } from "../lib/tauri.desktop";
  import UserIcon from "../../src-shared/UserIcon.svelte";
  import PassIcon from "../../src-shared/PassIcon.svelte";
  let {
    apiUrl = "",
    onActivated,
  }: { apiUrl?: string; onActivated: () => void } = $props();
  let code = $state("");
  let terminalName = $state("");
  let busy = $state(false);
  let error = $state("");
  let activationCodeInput: HTMLInputElement;

  onMount(() => activationCodeInput?.focus());
  async function activate() {
    error = "";
    busy = true;
    try {
      await claimActivation(code, terminalName, apiUrl);
      onActivated();
    } catch (cause) {
      error =
        cause instanceof Error
          ? cause.message
          : "Não foi possível ativar o terminal.";
    } finally {
      busy = false;
    }
  }
</script>

<main class="activation-shell">
  <section class="activation-card">
    <div class="mark">_,CTRL</div>
    <p class="eyebrow">CommerceCTRL PDV</p>
    <h1>Ative este terminal</h1>
    <p class="intro">
      Use o código de uso único fornecido pelo administrador e identifique este
      caixa. As vendas continuam locais mesmo quando a internet oscilar após a
      ativação.
    </p>
    <form
      onsubmit={(event) => {
        event.preventDefault();
        activate();
      }}
    >
      <label
        ><span><UserIcon size={15} /> Código de ativação</span><input
          bind:this={activationCodeInput}
          required
          maxlength="64"
          placeholder="CC-XXXXXXXXXXXX"
          bind:value={code}
        /></label
      >
      <label
        ><span><PassIcon size={15} /> Nome do terminal</span><input
          required
          minlength="2"
          maxlength="120"
          placeholder="Caixa 01"
          bind:value={terminalName}
        /></label
      >
      {#if !apiUrl}<p class="hint">
          O servidor configurado no instalador será usado.
        </p>{/if}
      {#if error}<p class="error" role="alert">{error}</p>{/if}
      <button disabled={busy}
        >{busy ? "Validando ativação…" : "Ativar terminal"}</button
      >
    </form>
    <small
      >O código não é salvo neste computador e não poderá ser reutilizado.</small
    >
  </section>
</main>

<style>
  .activation-shell {
    min-height: 100vh;
    display: grid;
    place-items: center;
    padding: 24px;
    background: var(--paper);
    color: var(--ink);
  }
  .activation-card {
    width: min(470px, 100%);
    padding: 34px;
    border: 1px solid var(--line);
    border-radius: 20px;
    background: var(--surface);
    box-shadow: var(--shadow-lifted);
  }
  .mark {
    color: var(--primary);
    font: 900 25px/1 "Space Grotesk", monospace;
    letter-spacing: -0.12em;
  }
  .eyebrow {
    margin: 25px 0 6px;
    color: var(--muted);
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  .activation-card h1 {
    margin: 0;
    font: 700 30px/1 "Space Grotesk", sans-serif;
    color: var(--ink);
  }
  .intro {
    color: var(--muted);
    font-size: 14px;
    line-height: 1.55;
  }
  .activation-card form {
    display: grid;
    gap: 16px;
    margin-top: 25px;
  }
  .activation-card label {
    display: grid;
    gap: 7px;
    color: var(--muted);
    font-size: 12px;
    font-weight: 700;
  }
  .activation-card label span,
  .hint {
    display: flex;
    gap: 7px;
    align-items: center;
  }
  .activation-card input {
    padding: 13px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--surface);
    color: var(--ink);
    font: 600 15px Inter, sans-serif;
    outline: 0;
  }
  .activation-card input:focus {
    border-color: var(--primary);
  }
  .activation-card button {
    padding: 14px;
    border: 0;
    border-radius: 10px;
    background: var(--primary);
    color: #fff;
    font-weight: 900;
    font-family: "Space Grotesk", sans-serif;
    cursor: pointer;
  }
  .activation-card button:hover:not(:disabled) {
    background: var(--primary-dark);
  }
  .activation-card button:disabled {
    opacity: 0.65;
  }
  .hint,
  .activation-card small {
    color: var(--muted);
    font-size: 11px;
  }
  .error {
    margin: 0;
    padding: 10px;
    border-radius: 8px;
    background: var(--danger-wash);
    color: var(--danger-ink);
    font-size: 12px;
  }
</style>
