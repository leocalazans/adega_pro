<script lang="ts">
  import { onMount } from "svelte";
  import { DollarSign, ShoppingCart, UserPlus, Package, TrendingUp } from "@lucide/svelte";
  import { dashboardSummary, listProducts } from "../lib/tauri";
  import { brl } from "../lib/format";
  import type { DashboardSummary, Product } from "../lib/types";
  let summary = $state<DashboardSummary | null>(null);
  let products = $state<Product[]>([]);
  let error = $state("");
  onMount(async () => {
    try { [summary, products] = await Promise.all([dashboardSummary(), listProducts(100)]); }
    catch (cause) { error = cause instanceof Error ? cause.message : "Falha ao carregar dashboard"; }
  });
  let lowStock = $derived(products.filter((p) => p.stock_qty < p.min_stock).sort((a,b) => a.stock_qty/a.min_stock-b.stock_qty/b.min_stock).slice(0,5));
  let chartMax = $derived(Math.max(1, ...(summary?.daily_sales ?? [1])));
</script>

{#if error}<div class="card card-body"><span class="badge danger">{error}</span></div>
{:else if !summary}<div class="card card-body">Carregando indicadores locais...</div>
{:else}
<div class="section-stack">
  <section class="grid grid-4">
    {#each [
      { title:"Vendas Hoje", value:brl(summary.sales_today_brl_cents), note:"Dados do SQLite local", icon:DollarSign },
      { title:"Itens Vendidos", value:String(summary.items_today), note:"No dia atual", icon:ShoppingCart },
      { title:"Novos Clientes", value:String(summary.new_customers_today), note:"Cadastros de hoje", icon:UserPlus },
      { title:"Estoque Baixo", value:`${summary.low_stock_count} itens`, note:"Abaixo do mínimo", icon:Package }
    ] as stat}
      <article class="card card-body"><div class="toolbar"><div><p class="small muted">{stat.title}</p><p class="metric">{stat.value}</p><p class="small muted">{stat.note}</p></div><span class="spacer"></span><span class="stat-icon"><stat.icon size={19}/></span></div></article>
    {/each}
  </section>
  <section class="grid grid-2 dashboard-main">
    <article class="card"><div class="card-header"><div><h2>Visão Geral de Vendas</h2><p class="small muted">Últimos 7 dias</p></div><span class="badge success"><TrendingUp size={13}/> Local</span></div><div class="card-body chart">{#each summary.daily_sales as value,index}<div class="bar-column"><span style={`height:${Math.max(2,value/chartMax*100)}%`}></span><small>{["D-6","D-5","D-4","D-3","D-2","Ontem","Hoje"][index]}</small><b>{brl(value)}</b></div>{/each}</div></article>
    <article class="card"><div class="card-header"><div><h2>Estoque crítico</h2><p class="small muted">Prioridade de reposição</p></div></div><div class="table-wrap"><table><thead><tr><th>Produto</th><th>Atual / mínimo</th></tr></thead><tbody>{#each lowStock as product}<tr><td><strong>{product.description}</strong></td><td><span class="badge danger">{product.stock_qty} / {product.min_stock}</span></td></tr>{/each}{#if !lowStock.length}<tr><td colspan="2" class="muted">Nenhum produto abaixo do mínimo.</td></tr>{/if}</tbody></table></div></article>
  </section>
</div>
{/if}
<style>.dashboard-main{grid-template-columns:1.45fr 1fr}.chart{height:280px;display:flex;align-items:end;gap:10px;padding-top:30px}.bar-column{flex:1;height:100%;display:flex;flex-direction:column;justify-content:end;align-items:center;gap:6px}.bar-column span{width:min(46px,80%);border-radius:7px 7px 2px 2px;background:linear-gradient(180deg,#7b67b7,#4e59b5)}.bar-column small{color:#737993;font-size:9px}.bar-column b{font-size:8px;color:#555d78}@media(max-width:850px){.dashboard-main{grid-template-columns:1fr}}</style>
