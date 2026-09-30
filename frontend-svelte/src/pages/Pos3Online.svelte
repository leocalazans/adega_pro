<script lang="ts">
  import Pos2 from "./Pos2.svelte"; import DeliveryOrderBar from "./pos3/DeliveryOrderBar.svelte"; import DeliveryDrawer from "./pos3/DeliveryDrawer.svelte"; import type{DeliveryOrder}from"./pos3/types";
  let orders=$state<DeliveryOrder[]>([{id:"1042",provider:"iFood",state:"new",elapsed:"agora",pickupCode:"4821",items:[{name:"Cerveja Pilsen 350 ml",ean:"78900001",checked:false},{name:"Água tônica",ean:"78900003",checked:false}]},{id:"087",provider:"99Food",state:"preparing",elapsed:"4 min",pickupCode:"9908",items:[{name:"Gin dry 750 ml",ean:"78900005",checked:true},{name:"Gelo em cubos 3 kg",ean:"78900008",checked:false}]},{id:"551",provider:"Zé Delivery",state:"ready",elapsed:"9 min",pickupCode:"7355",items:[{name:"Vinho tinto suave",ean:"78900002",checked:true},{name:"Amendoim salgado",ean:"78900007",checked:true}]}]); let selected=$state<DeliveryOrder|null>(null); function save(order:DeliveryOrder){orders=orders.map(item=>item.id===order.id?order:item)}
</script>
<div class="pos3-host"><Pos2/><div class="order-bar"><DeliveryOrderBar {orders} onOpen={(order)=>selected=order}/></div>{#if selected}<DeliveryDrawer order={selected} onSave={save} onClose={()=>selected=null}/>{/if}</div>
<style>
  .pos3-host{position:relative;height:100dvh;overflow:hidden}
  .order-bar{position:absolute;z-index:8;top:164px;left:calc(5rem + 22px);right:calc(5rem + 430px)}
  .pos3-host :global(.pos2 .section-title){margin-top:124px}
  @media(max-width:820px){
    .order-bar{right:calc(5rem + 22px)}
    .pos3-host :global(.pos2 .section-title){margin-top:138px}
  }
</style>
