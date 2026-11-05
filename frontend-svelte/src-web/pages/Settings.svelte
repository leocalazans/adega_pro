<script lang="ts">
  import { ImageUp } from "@lucide/svelte";
  import { cloudOwnerBranding, cloudSaveOwnerBranding } from "../lib/tauri.web";
  import type { TenantBranding } from "../../src-shared/types";

  let storeName = $state("Mercadinho Martins");
  let storeLogo = $state("");
  let brandMessage = $state("");
  let brandColor = $state("#0D5C3A");
  let busy = $state(false);

  async function load() {
    try { const saved = JSON.parse(localStorage.getItem("commercectrl.client.branding") ?? "null"); if (saved) { storeName = saved.name || storeName; storeLogo = saved.logo || ""; brandColor = saved.primary_color || brandColor; } } catch {}
    try {
      const branding = await cloudOwnerBranding();
      storeName = branding.display_name;
      storeLogo = branding.logo_data_url || "";
      brandColor = branding.primary_color || "#0D5C3A";
      const cached = { name: storeName, logo: storeLogo, primary_color: brandColor };
      localStorage.setItem("commercectrl.client.branding", JSON.stringify(cached));
      window.dispatchEvent(new CustomEvent("commercectrl-branding", { detail: cached }));
    } catch {}
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
    const branding = { name: storeName.trim() || "Mercadinho Martins", logo: storeLogo, primary_color: brandColor || "#0D5C3A" };
    busy = true; brandMessage = "";
    try {
      const saved = await cloudSaveOwnerBranding(branding.name, branding.logo, branding.primary_color);
      const cached = { name: saved.display_name, logo: saved.logo_data_url || "", primary_color: saved.primary_color || "#0D5C3A" };
      localStorage.setItem("commercectrl.client.branding", JSON.stringify(cached));
      window.dispatchEvent(new CustomEvent("commercectrl-branding", { detail: cached }));
      brandMessage = "Identidade salva no tenant e sincronizada neste terminal.";
    } catch (error) { brandMessage = error instanceof Error ? error.message : String(error); }
    finally { busy = false; }
  }
  onMount(load);
</script>

<div class="page-stack">
  <article class="card"><div class="card-header"><div><h2>Identidade do cliente</h2><p class="small muted">Nome, logo e cor da marca exibidos no painel web.</p></div><ImageUp size={20}/></div><div class="card-body branding"><label>Nome do mercado<input class="input" maxlength="80" bind:value={storeName}/></label><label>Cor primária<input class="input" type="color" bind:value={brandColor}/></label><label>Logo do mercado<input class="input" type="file" accept="image/png,image/jpeg,image/webp,image/svg+xml" onchange={chooseLogo}/></label>{#if storeLogo}<div class="logo-preview"><img src={storeLogo} alt="Prévia da logo"/><button class="button secondary" onclick={()=>storeLogo=""}>Remover imagem</button></div>{/if}<button class="button" disabled={busy} onclick={saveBranding}>Salvar identidade</button>{#if brandMessage}<p class="notice">{brandMessage}</p>{/if}</div></article>
</div>

<style>.branding{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:14px;align-items:end}.branding label{display:grid;gap:6px;font-size:12px;font-weight:700}.logo-preview{grid-column:1/-1;display:flex;align-items:center;gap:14px}.logo-preview img{width:96px;height:72px;object-fit:contain;border:1px solid var(--line);border-radius:10px;background:#fff}@media(max-width:700px){.branding{grid-template-columns:1fr}}</style>
