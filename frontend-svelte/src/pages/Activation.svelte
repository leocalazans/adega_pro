<script lang="ts">
  import { KeyRound, MonitorCheck, Wifi } from "@lucide/svelte";
  import { claimActivation } from "../lib/tauri";
  let { apiUrl = "", onActivated }: { apiUrl?: string; onActivated: () => void } = $props();
  let code = $state("");
  let terminalName = $state("");
  let busy = $state(false);
  let error = $state("");
  async function activate() {
    error = ""; busy = true;
    try { await claimActivation(code, terminalName, apiUrl); onActivated(); }
    catch (cause) { error = cause instanceof Error ? cause.message : "Não foi possível ativar o terminal."; }
    finally { busy = false; }
  }
</script>

<main class="activation-shell"><section class="activation-card">
  <div class="mark">_,CTRL</div><p class="eyebrow">CommerceCTRL PDV</p><h1>Ative este terminal</h1>
  <p class="intro">Use o código de uso único fornecido pelo administrador e identifique este caixa. As vendas continuam locais mesmo quando a internet oscilar após a ativação.</p>
  <form onsubmit={(event)=>{event.preventDefault();activate()}}>
    <label><span><KeyRound size={15}/> Código de ativação</span><input autofocus required maxlength="64" placeholder="CC-XXXXXXXXXXXX" bind:value={code}/></label>
    <label><span><MonitorCheck size={15}/> Nome do terminal</span><input required minlength="2" maxlength="120" placeholder="Caixa 01" bind:value={terminalName}/></label>
    {#if !apiUrl}<p class="hint"><Wifi size={14}/> O servidor configurado no instalador será usado.</p>{/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <button disabled={busy}>{busy ? "Validando ativação…" : "Ativar terminal"}</button>
  </form>
  <small>O código não é salvo neste computador e não poderá ser reutilizado.</small>
</section></main>
<style>
.activation-shell{min-height:100vh;display:grid;place-items:center;padding:24px;background:repeating-linear-gradient(135deg,#111 0 12px,#171717 12px 24px);color:#f4f6f4}.activation-card{width:min(470px,100%);padding:34px;border:1px solid #343b35;border-radius:20px;background:#0a0c0a;box-shadow:0 24px 70px #0009}.mark{color:#92d66d;font:900 25px/1 monospace;letter-spacing:-.12em}.eyebrow{margin:25px 0 6px;color:#91a095;font-size:11px;font-weight:800;letter-spacing:.12em;text-transform:uppercase}.activation-card h1{margin:0;font-size:30px}.intro{color:#aab3ac;font-size:14px;line-height:1.55}.activation-card form{display:grid;gap:16px;margin-top:25px}.activation-card label{display:grid;gap:7px;color:#dce4de;font-size:12px;font-weight:700}.activation-card label span,.hint{display:flex;gap:7px;align-items:center}.activation-card input{padding:13px;border:1px solid #3a453d;border-radius:10px;background:#151915;color:#fff;font:600 15px monospace;outline:0}.activation-card input:focus{border-color:#92d66d}.activation-card button{padding:14px;border:0;border-radius:10px;background:#92d66d;color:#102010;font-weight:900;cursor:pointer}.activation-card button:disabled{opacity:.65}.hint,.activation-card small{color:#9baa9e;font-size:11px}.error{margin:0;padding:10px;border-radius:8px;background:#411f22;color:#ffc6c9;font-size:12px}
</style>
