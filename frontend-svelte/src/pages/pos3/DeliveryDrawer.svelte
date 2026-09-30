<script lang="ts">
  import type { DeliveryOrder } from "./types";

  let {
    order,
    onSave,
    onClose
  }: {
    order: DeliveryOrder;
    onSave: (order: DeliveryOrder) => void;
    onClose: () => void;
  } = $props();

  function copyOrder(value: DeliveryOrder): DeliveryOrder {
    return { ...value, items: value.items.map((item) => ({ ...item })) };
  }

  // A gaveta é remontada a cada abertura; o rascunho precisa nascer completo
  // para que nenhum frame intermediário ofereça ações para um pedido vazio.
  let draft = $state<DeliveryOrder>(copyOrder(order));
  let code = $state("");

  function check(index: number) {
    draft.items[index].checked = true;
    draft.state = "preparing";
    draft = { ...draft };
  }

  function save() {
    onSave(draft);
    onClose();
  }
</script>

<svelte:window onkeydown={(event) => event.key === "Escape" && onClose()} />

<div class="veil" onclick={onClose} role="presentation"></div>
<div class="drawer" role="dialog" aria-modal="true" aria-label={`Pedido ${draft.provider} ${draft.id}`}>
  <header>
    <div><small>{draft.provider}</small><h2>Pedido #{draft.id}</h2></div>
    <button onclick={save}>Continuar depois</button>
  </header>
  <p>A venda local permanece aberta enquanto este pedido é preparado.</p>
  <section>
    {#each draft.items as item, index}
      <article class:done={item.checked}>
        <span><strong>{item.name}</strong><small>{item.ean}</small></span>
        <button disabled={item.checked} onclick={() => check(index)}>{item.checked ? "Conferido" : "Confirmar bip"}</button>
      </article>
    {/each}
  </section>
  {#if draft.items.every((item) => item.checked) && draft.state !== "ready"}
    <button class="primary" onclick={() => draft = { ...draft, state: "ready" }}>Pedido pronto</button>
  {/if}
  {#if draft.state === "ready"}
    <label>Código do entregador<input bind:value={code} placeholder="Informe ou bipe o código" /></label>
    <button class="primary" disabled={code !== draft.pickupCode} onclick={() => { draft = { ...draft, state: "collected" }; save(); }}>Confirmar retirada</button>
  {/if}
</div>

<style>
  .veil{position:fixed;inset:0;background:#092e3455;z-index:40}.drawer{position:fixed;z-index:41;right:0;top:0;bottom:0;width:min(430px,92vw);background:#f2e9d8;color:#092e34;padding:28px;box-shadow:-12px 0 #e7a83a;display:flex;flex-direction:column;gap:18px;animation:enter .18s ease-out}.drawer header{display:flex;justify-content:space-between;align-items:start}.drawer h2{font:700 28px "Space Grotesk";margin:3px 0}.drawer header button{border:0;background:transparent;color:#31594e;font-weight:800}.drawer>p{font-size:13px;color:#617068}.drawer section{display:grid;gap:8px}.drawer article{display:flex;justify-content:space-between;align-items:center;padding:12px;border:1px solid #c8bda7;background:#fffdf8}.drawer article.done{border-left:5px solid #3e8a69}.drawer article strong,.drawer article small{display:block}.drawer article small{font-size:10px;color:#68736d}.drawer article button,.primary{border:0;background:#092e34;color:white;padding:10px;font-weight:800}.drawer article button:disabled{background:#d7dfd8;color:#517064}.drawer label{display:grid;gap:7px;font-size:12px;font-weight:800}.drawer input{padding:12px;border:1px solid #9eaa9d}.primary{margin-top:auto}.primary:disabled{opacity:.4}.drawer button:focus-visible,.drawer input:focus-visible{outline:3px solid #e7a83a;outline-offset:2px}@keyframes enter{from{transform:translateX(100%)}to{transform:none}}@media(prefers-reduced-motion:reduce){.drawer{animation:none}}
</style>
