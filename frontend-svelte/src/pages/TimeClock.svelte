<script lang="ts">
  import { onMount } from "svelte";
  import { Clock3, Coffee, LogIn, LogOut, RefreshCw } from "@lucide/svelte";
  import { listEmployees, listTimeEntries, recordTimeEntry } from "../lib/tauri";
  import type { Employee, TimeEntry } from "../lib/types";
  let employees=$state<Employee[]>([]), entries=$state<TimeEntry[]>([]), employeeId=$state(0), note=$state(""), busy=$state(false), message=$state("");
  const labels:Record<TimeEntry["event_type"],string>={clock_in:"Entrada",break_start:"Início do intervalo",break_end:"Fim do intervalo",clock_out:"Saída"};
  async function load(){try{[employees,entries]=await Promise.all([listEmployees(),listTimeEntries()]);employees=employees.filter(e=>e.status==="Ativo");if(!employeeId&&employees[0])employeeId=employees[0].id;message=""}catch(e){message=e instanceof Error?e.message:"Falha ao carregar o ponto"}}
  async function punch(type:TimeEntry["event_type"]){if(!employeeId)return;busy=true;try{await recordTimeEntry(employeeId,type,note);note="";await load()}catch(e){message=e instanceof Error?e.message:"Falha ao registrar"}finally{busy=false}}
  onMount(()=>{void load()});
</script>
<section class="page-stack">
  <article class="card"><div class="card-header"><div><h2><Clock3 size={20}/> Registro de ponto</h2><p class="small muted">Funciona offline e sincroniza automaticamente quando a conexão voltar.</p></div><button class="icon-button" onclick={load} title="Atualizar"><RefreshCw size={18}/></button></div>
    <div class="card-body form-grid"><label class="wide">Funcionário<select bind:value={employeeId}>{#each employees as employee}<option value={employee.id}>{employee.name} · {employee.role}</option>{/each}</select></label><label class="wide">Observação opcional<input class="input" maxlength="180" bind:value={note}/></label>{#if message}<p class="wide error-text">{message}</p>{/if}<div class="wide punch-grid"><button class="button" disabled={busy||!employeeId} onclick={()=>punch("clock_in")}><LogIn size={18}/> Entrada</button><button class="button secondary" disabled={busy||!employeeId} onclick={()=>punch("break_start")}><Coffee size={18}/> Iniciar intervalo</button><button class="button secondary" disabled={busy||!employeeId} onclick={()=>punch("break_end")}><Coffee size={18}/> Voltar</button><button class="button danger" disabled={busy||!employeeId} onclick={()=>punch("clock_out")}><LogOut size={18}/> Saída</button></div></div>
  </article>
  <article class="card"><div class="card-header"><div><h2>Registros recentes</h2><p class="small muted">Histórico local desta unidade.</p></div></div><div class="table-wrap"><table><thead><tr><th>Data e hora</th><th>Funcionário</th><th>Evento</th><th>Observação</th></tr></thead><tbody>{#each entries as entry}<tr><td>{new Date(entry.occurred_at*1000).toLocaleString("pt-BR")}</td><td><strong>{entry.employee_name}</strong></td><td><span class="badge">{labels[entry.event_type]}</span></td><td>{entry.note||"—"}</td></tr>{:else}<tr><td colspan="4" class="muted">Nenhum ponto registrado.</td></tr>{/each}</tbody></table></div></article>
</section>
<style>.card-header h2{display:flex;align-items:center;gap:8px}.punch-grid{display:grid;grid-template-columns:repeat(4,minmax(150px,1fr));gap:10px}.danger{background:#b91c1c}@media(max-width:850px){.punch-grid{grid-template-columns:1fr 1fr}}</style>
