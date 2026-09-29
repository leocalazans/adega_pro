<script lang="ts">
  import { onMount } from "svelte";
  import { ImageUp, KeyRound, Printer, RefreshCw, Usb } from "@lucide/svelte";
  import { cloudOwnerBranding, cloudSaveOwnerBranding, createTerminalBackup, getPrinterConfig, importLicense, licenseStatus, listPrinters, refreshLicense, savePrinterConfig, scanLicenseMedia, scheduleTerminalRestore, testPrinter, type PrinterConfig, type PrinterInfo } from "../lib/tauri";
  import type { LicenseStatus } from "../lib/types";

  let license = $state<LicenseStatus | null>(null);
  let path = $state("");
  let busy = $state(false);
  let message = $state("");
  let storeName = $state("Mercadinho Martins");
  let storeLogo = $state("");
  let brandMessage = $state("");
  let printers = $state<PrinterInfo[]>([]);
  let printer = $state<PrinterConfig>({mode:"auto",windows_printer:null,escpos_host:null,escpos_port:9100,local_port:null,prefer_escpos:true,print_logo:true});
  let printerMessage = $state("");
  let updateMessage = $state("");
  let restorePath=$state(""), backupMessage=$state("");

  async function load() {
    license = await licenseStatus();
  }
  async function scan() {
    busy = true; message = "";
    try { license = await scanLicenseMedia(); message = "Licença do pendrive importada e validada."; }
    catch (error) { message = error instanceof Error ? error.message : String(error); }
    finally { busy = false; }
  }
  async function importPath() {
    if (!path.trim()) return;
    busy = true; message = "";
    try { license = await importLicense(path.trim()); message = "Licença importada e validada."; }
    catch (error) { message = error instanceof Error ? error.message : String(error); }
    finally { busy = false; }
  }
  async function renew() {
    busy = true; message = "";
    try { license = await refreshLicense(); message = "Licença renovada pelo servidor."; }
    catch (error) { message = error instanceof Error ? error.message : String(error); }
    finally { busy = false; }
  }
  async function chooseLogo(event: Event) {
    const file = (event.currentTarget as HTMLInputElement).files?.[0];
    if (!file) return;
    if (!file.type.startsWith("image/") || file.size > 2_000_000) { brandMessage = "Use PNG, JPG, WebP ou SVG de até 2 MB."; return; }
    try {
      const bitmap = await createImageBitmap(file);
      const scale = Math.min(1, 512 / Math.max(bitmap.width, bitmap.height));
      const canvas = document.createElement("canvas");
      canvas.width = Math.max(1, Math.round(bitmap.width * scale));
      canvas.height = Math.max(1, Math.round(bitmap.height * scale));
      canvas.getContext("2d", { alpha: true })?.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
      bitmap.close();
      const blob = await new Promise<Blob>((resolve, reject) => canvas.toBlob(value => value ? resolve(value) : reject(new Error("Este navegador não oferece conversão WebP")), "image/webp", .86));
      storeLogo = await new Promise<string>((resolve, reject) => { const reader=new FileReader(); reader.onload=()=>resolve(String(reader.result)); reader.onerror=()=>reject(reader.error); reader.readAsDataURL(blob); });
      brandMessage = `Logo convertida para WebP (${Math.ceil(blob.size/1024)} KB), pronta para sincronizar.`;
    } catch (error) { brandMessage = error instanceof Error ? error.message : String(error); }
  }
  async function saveBranding() {
    const branding = { name: storeName.trim() || "Mercadinho Martins", logo: storeLogo };
    busy = true; brandMessage = "";
    try {
      const saved = await cloudSaveOwnerBranding(branding.name, branding.logo);
      const cached = { name: saved.display_name, logo: saved.logo_data_url || "" };
      localStorage.setItem("commercectrl.client.branding", JSON.stringify(cached));
      window.dispatchEvent(new CustomEvent("commercectrl-branding", { detail: cached }));
      brandMessage = "Identidade salva no tenant e sincronizada neste terminal.";
    } catch (error) { brandMessage = error instanceof Error ? error.message : String(error); }
    finally { busy = false; }
  }
  async function savePrinting() { busy=true;printerMessage="";try{await savePrinterConfig(printer);printerMessage="Configuração de impressão salva neste terminal."}catch(e){printerMessage=e instanceof Error?e.message:String(e)}finally{busy=false} }
  async function printTest(){printerMessage="";try{await savePrinterConfig(printer);await testPrinter();printerMessage="Teste enviado para a fila de impressão."}catch(e){printerMessage=e instanceof Error?e.message:String(e)}}
  async function installUpdate(){busy=true;updateMessage="Procurando atualização assinada…";try{const {check}=await import("@tauri-apps/plugin-updater");const update=await check();if(!update){updateMessage="Este terminal já está na versão mais recente.";return}updateMessage=`Baixando versão ${update.version}…`;await update.downloadAndInstall();updateMessage="Atualização instalada. Reiniciando…";const {relaunch}=await import("@tauri-apps/plugin-process");await relaunch()}catch(e){updateMessage=e instanceof Error?e.message:String(e)}finally{busy=false}}
  async function backup(){try{backupMessage=`Backup verificado: ${await createTerminalBackup()}`}catch(e){backupMessage=String(e)}}
  async function restore(){if(!restorePath.trim())return;try{await scheduleTerminalRestore(restorePath.trim());backupMessage="Restauração preparada. Reinicie o aplicativo para aplicar."}catch(e){backupMessage=String(e)}}
  onMount(() => {
    try { const saved = JSON.parse(localStorage.getItem("commercectrl.client.branding") ?? "null"); if (saved) { storeName = saved.name || storeName; storeLogo = saved.logo || ""; } } catch {}
    cloudOwnerBranding().then((saved)=>{storeName=saved.display_name;storeLogo=saved.logo_data_url||"";const cached={name:storeName,logo:storeLogo};localStorage.setItem("commercectrl.client.branding",JSON.stringify(cached));window.dispatchEvent(new CustomEvent("commercectrl-branding",{detail:cached}))}).catch(()=>{});
    load().catch((error) => message = String(error));
    Promise.all([listPrinters(),getPrinterConfig()]).then(([found,saved])=>{printers=found;printer=saved}).catch(e=>printerMessage=String(e));
  });
</script>

<div class="page-stack">
  <article class="card"><div class="card-header"><div><h2>Atualização do aplicativo</h2><p class="small muted">Pacotes assinados publicados pelo canal oficial. Vendas locais não são apagadas durante a atualização.</p></div><RefreshCw size={20}/></div><div class="card-body"><button class="button" disabled={busy} onclick={installUpdate}>Buscar e instalar atualização</button>{#if updateMessage}<p class="notice">{updateMessage}</p>{/if}</div></article>
  <article class="card"><div class="card-header"><div><h2>Backup e restauração do terminal</h2><p class="small muted">O backup SQLite é validado antes de ser aceito. A restauração mantém uma cópia de segurança anterior.</p></div></div><div class="card-body settings-grid"><div class="wide"><button class="button" onclick={backup}>Criar backup agora</button></div><label class="wide">Arquivo de backup para restaurar<input class="input" placeholder="C:\backups\commercectrl-terminal.db" bind:value={restorePath}/></label><div class="wide"><button class="button secondary" disabled={!restorePath.trim()} onclick={restore}>Validar e preparar restauração</button></div>{#if backupMessage}<p class="wide notice">{backupMessage}</p>{/if}</div></article>
  <article class="card"><div class="card-header"><div><h2>Impressão</h2><p class="small muted">ESC/POS de rede é priorizada; a fila do Windows assume automaticamente se ela não responder.</p></div><Printer size={20}/></div><div class="card-body settings-grid">
    <label>Modo<select bind:value={printer.mode}><option value="auto">Automático ESC/POS</option><option value="windows">Fila RAW do Windows (ESC/POS)</option><option value="windows_graphic">Impressora comum / diálogo Windows</option><option value="escpos_network">ESC/POS de rede</option><option value="local_port">Porta local ESC/POS</option><option value="smartpos">Smart POS via conector</option></select></label>
    <label>Impressora Windows<select bind:value={printer.windows_printer}><option value={null}>Padrão do Windows</option>{#each printers as item}<option value={item.name}>{item.name}{item.is_default?" — padrão":""}{item.is_network?" — rede":""}</option>{/each}</select></label>
    <label>IP ou nome da ESC/POS<input class="input" placeholder="192.168.1.80" bind:value={printer.escpos_host}/></label><label>Porta TCP<input class="input" type="number" min="1" max="65535" bind:value={printer.escpos_port}/></label>
    <label>Porta local<input class="input" placeholder="USB001, LPT1 ou \\servidor\impressora" bind:value={printer.local_port}/></label>
    <label class="check"><input type="checkbox" bind:checked={printer.prefer_escpos}/> Preferir ESC/POS de rede quando disponível</label><label class="check"><input type="checkbox" bind:checked={printer.print_logo}/> Imprimir logo do mercado</label>
    {#if printer.mode==="smartpos"}<p class="wide notice">Smart POS exige o aplicativo/SDK do adquirente instalado. Este conector será ativado ao informar o modelo e fabricante do equipamento.</p>{/if}
    <div class="wide actions"><button class="button" disabled={busy} onclick={savePrinting}>Salvar impressão</button><button class="button secondary" onclick={printTest}>Imprimir teste</button></div>{#if printerMessage}<p class="wide notice">{printerMessage}</p>{/if}
  </div></article>
  <article class="card"><div class="card-header"><div><h2>Identidade do cliente</h2><p class="small muted">Nome e logo exibidos no PDV deste terminal.</p></div><ImageUp size={20}/></div><div class="card-body branding"><label>Nome do mercado<input class="input" maxlength="80" bind:value={storeName}/></label><label>Logo do mercado<input class="input" type="file" accept="image/png,image/jpeg,image/webp,image/svg+xml" onchange={chooseLogo}/></label>{#if storeLogo}<div class="logo-preview"><img src={storeLogo} alt="Prévia da logo"/><button class="button secondary" onclick={()=>storeLogo=""}>Remover imagem</button></div>{/if}<button class="button" onclick={saveBranding}>Salvar identidade</button>{#if brandMessage}<p class="notice">{brandMessage}</p>{/if}</div></article>
  <article class="card">
    <div class="card-header"><div><h2>Licenciamento</h2><p class="small muted">Validação assinada e operação offline com carência.</p></div><KeyRound size={20}/></div>
    <div class="card-body settings-grid">
      {#if license}
        <div><span class="small muted">Estado</span><p><span class:danger={!license.allowed_to_sell} class="badge">{license.mode}</span></p></div>
        <div><span class="small muted">Instalação</span><p><code>{license.installation_id}</code></p></div>
        <div class="wide"><span class="small muted">Situação</span><p>{license.message}</p></div>
        {#if license.expires_at}<div><span class="small muted">Validade</span><p>{new Date(license.expires_at*1000).toLocaleString("pt-BR")}</p></div>{/if}
        {#if license.grace_until}<div><span class="small muted">Fim da carência</span><p>{new Date(license.grace_until*1000).toLocaleString("pt-BR")}</p></div>{/if}
      {/if}
      <div class="wide actions"><button class="button" disabled={busy} onclick={renew}><RefreshCw size={16}/>Renovar on-line</button><button class="button secondary" disabled={busy} onclick={scan}><Usb size={16}/>Ler licença do pendrive</button><button class="button secondary" onclick={load}>Atualizar estado</button></div>
      <label class="wide">Ou informe o arquivo de licença<input class="input" placeholder="E:\commercectrl-license.json" bind:value={path}/></label>
      <div class="wide"><button class="button secondary" disabled={busy||!path.trim()} onclick={importPath}>Importar arquivo</button></div>
      {#if message}<p class="wide notice">{message}</p>{/if}
    </div>
  </article>
  <article class="card"><div class="card-header"><h2>Integrações externas</h2></div><div class="card-body"><p>PIX Asaas, NFC-e SP, iFood, 99Food e TEF permanecem independentes. A indisponibilidade de um provedor não apaga vendas, estoque ou histórico local.</p><p class="small muted">Estados externos só são marcados como concluídos após confirmação assinada ou resposta homologada.</p></div></article>
</div>

<style>.settings-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:18px}.wide{grid-column:1/-1}.actions{display:flex;gap:10px;flex-wrap:wrap}.notice{padding:10px;border-radius:8px;background:#eef0ff}.danger{background:#ffe9ea;color:#a9232e}.branding{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:14px;align-items:end}.branding label{display:grid;gap:6px;font-size:12px;font-weight:700}.logo-preview{grid-column:1/-1;display:flex;align-items:center;gap:14px}.logo-preview img{width:96px;height:72px;object-fit:contain;border:1px solid var(--line);border-radius:10px;background:#fff}@media(max-width:700px){.settings-grid,.branding{grid-template-columns:1fr}}</style>
