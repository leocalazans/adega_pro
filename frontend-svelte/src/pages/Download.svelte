<script lang="ts">
  import { CheckCircle2, Download, MonitorDown, ShieldCheck, WifiOff } from "@lucide/svelte";

  let { onBack }: { onBack: () => void } = $props();
  const installerUrl = (import.meta.env.VITE_DESKTOP_DOWNLOAD_URL as string | undefined)?.trim();
  const available = Boolean(installerUrl);
</script>

<main class="download-page">
  <header>
    <a class="wordmark" href="/">_,CTRL</a>
    <button onclick={onBack}>Voltar ao site</button>
  </header>

  <section class="download-hero">
    <div class="stamp"><MonitorDown size={31}/><span>Windows</span></div>
    <p class="kicker">Aplicativo de caixa</p>
    <h1>Instale onde a venda acontece.</h1>
    <p class="lead">O CommerceCTRL PDV guarda a operação localmente e sincroniza com a nuvem quando a conexão voltar.</p>
    {#if available}
      <a class="download-button" href={installerUrl}><Download size={20}/>Baixar CommerceCTRL para Windows</a>
      <p class="fine">Ao concluir o download, execute o instalador e entre com o usuário liberado para a sua loja.</p>
    {:else}
      <div class="release-pending">
        <strong>Instalador em publicação</strong>
        <span>A assinatura pode ser ativada agora; o botão de download será liberado assim que a primeira versão assinada for publicada.</span>
      </div>
    {/if}
  </section>

  <section class="install-notes" aria-label="Como funciona">
    <article><WifiOff size={21}/><div><h2>Continua atendendo offline</h2><p>Produtos, vendas e caixa ficam no terminal; a fila sincroniza depois.</p></div></article>
    <article><ShieldCheck size={21}/><div><h2>Atualização verificada</h2><p>O app consulta somente o canal oficial de atualizações assinado.</p></div></article>
    <article><CheckCircle2 size={21}/><div><h2>Próxima abertura</h2><p>Use o atalho CommerceCTRL PDV e faça o login do funcionário.</p></div></article>
  </section>
</main>

<style>
  .download-page{--ink:#17251f;--forest:#153e31;--lime:#dced75;--paper:#f5f1e8;min-height:100vh;background:var(--paper);color:var(--ink);font-family:Inter,system-ui,sans-serif}.download-page header{height:76px;padding:0 clamp(22px,6vw,88px);display:flex;align-items:center;justify-content:space-between;border-bottom:1px solid #cfc7b5}.wordmark{font:900 22px "Space Grotesk";letter-spacing:-.12em;color:var(--forest);text-decoration:none}.download-page header button{border:0;background:transparent;color:var(--forest);font-weight:750;cursor:pointer}.download-hero{padding:clamp(60px,11vw,160px) clamp(22px,10vw,160px) clamp(54px,9vw,110px);max-width:930px}.stamp{width:max-content;display:flex;gap:9px;align-items:center;padding:8px 11px;background:var(--lime);color:var(--forest);font-size:12px;font-weight:850}.kicker{margin:39px 0 12px;font-size:12px;font-weight:850;color:#3a6851}.download-hero h1{max-width:720px;margin:0;font:700 clamp(48px,7vw,88px)/.91 "Space Grotesk",sans-serif;letter-spacing:-.08em}.lead{max-width:600px;font-size:19px;line-height:1.48;color:#506258;margin:28px 0}.download-button{width:max-content;display:inline-flex;align-items:center;gap:10px;text-decoration:none;background:var(--forest);color:white;padding:16px 20px;font-size:15px;font-weight:800}.fine{font-size:12px;color:#68756c;max-width:480px;line-height:1.45;margin-top:14px}.release-pending{max-width:620px;padding:18px 19px;border-left:4px solid #e04b2f;background:#fff5ed;display:grid;gap:5px}.release-pending strong{font:800 17px "Space Grotesk"}.release-pending span{font-size:14px;line-height:1.45;color:#5d665f}.install-notes{border-top:1px solid #cfc7b5;display:grid;grid-template-columns:repeat(3,1fr)}.install-notes article{min-height:180px;padding:31px clamp(22px,4vw,67px);display:flex;gap:13px;border-right:1px solid #cfc7b5}.install-notes article:last-child{border:0}.install-notes :global(svg){color:#1f8a56;flex:none}.install-notes h2{font:750 18px "Space Grotesk";letter-spacing:-.03em;margin:0 0 7px}.install-notes p{font-size:13px;line-height:1.5;color:#5d695f;margin:0}@media(max-width:700px){.download-hero{padding:65px 24px}.install-notes{grid-template-columns:1fr}.install-notes article{min-height:auto;border-right:0;border-bottom:1px solid #cfc7b5}.download-button{width:100%;justify-content:center}}
</style>
