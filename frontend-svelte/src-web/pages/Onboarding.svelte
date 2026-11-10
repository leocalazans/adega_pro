<script lang="ts">
  import { Building2, CheckCircle2, ChevronLeft, ChevronRight, ImageUp, MapPin, PackagePlus, ShieldCheck, QrCode, Smartphone, Link2 } from "@lucide/svelte";
  import { signupTenant } from "../lib/tauri.web";
  import type { TenantPaymentIntentOut } from "../lib/tauri.web";
  let { onDone }: { onDone: () => void } = $props();
  let step=$state(0), saving=$state(false), message=$state(""), plan=$state("starter");
  let storeName=$state(""), legalName=$state(""), cnpj=$state(""), ie=$state(""), email=$state(""), phone=$state(""), address=$state(""), unitName=$state("Matriz"), logo=$state("");
  let password=$state(""), passwordConfirm=$state("");
  const passwordValid=$derived(password.length>=8&&password.length<=128&&password===passwordConfirm);
  let paymentMethod=$state("pix"); // pix, credit_card, boleto
  let paymentResult=$state<TenantPaymentIntentOut | null>(null);
  let paymentError=$state("");
  let ownerEmail=$state(""), tenantSlug=$state("");
  let tenantCreatedId=$state<string | null>(null);
  const steps=["Empresa","Primeira unidade","Identidade","Plano e pagamento","Concluir"];
  const plans=[
    {id:"starter",name:"Começar",price:"R$ 100/mês",detail:"PDV, estoque e relatórios básicos · sem emissão fiscal"},
    {id:"fiscal",name:"Fiscal",price:"R$ 180/mês",detail:"Inclui recursos fiscais após homologação por UF"},
    {id:"premium",name:"PDV Premium",price:"R$ 250/mês",detail:"Multiunidade, TV de ofertas e recursos premium"},
  ];
  const paymentOptions=[
    {id:"pix",name:"PIX",detail:"Pagamento à vista via QR Code ou Copia e Cola",icon:"<path d=\"M21 12a9 9 0 01-9-9V4.5l3-3m0 0l-3 3m3-3v13.5A9 9 0 013 21c0 4.971 4.029 9 9 9s9-4.029 9-9v-10.5M3 12l3 3m0 0l3-3m-3 3v13.5\" stroke=\"#22c55e\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" fill=\"none\"/>"},
    {id:"credit_card",name:"Cartão de crédito",detail:"Parcelado em até 12x",icon:"<rect width=\"18\" height=\"12\" x=\"3\" y=\"6\" rx=\"2\" ry=\"2\" stroke=\"#3b82f6\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" fill=\"none\"/><line x1=\"3\" y1=\"10\" x2=\"21\" y2=\"10\" stroke=\"#3b82f6\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/><circle cx=\"12\" cy=\"6\" r=\"1\" fill=\"#3b82f6\"/>"},
    {id:"boleto",name:"Boleto bancário",detail:"Vencimento em até 7 dias",icon:"<rect width=\"18\" height=\"12\" x=\"3\" y=\"6\" rx=\"2\" ry=\"2\" stroke=\"#f59e0b\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" fill=\"none\"/><path d=\"M7 10h4M14 10h4M7 14h4M14 14h4\" stroke=\"#f59e0b\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" fill=\"none\"/><line x1=\"10\" y1=\"6\" x2=\"10\" y2=\"18\" stroke=\"#f59e0b\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" fill=\"none\"/>"}
  ];
  const desktopDownloadUrl = import.meta.env.VITE_DESKTOP_DOWNLOAD_URL as string | undefined;
  async function chooseLogo(event:Event){const file=(event.currentTarget as HTMLInputElement).files?.[0];if(!file)return;const bitmap=await createImageBitmap(file);const scale=Math.min(1,512/Math.max(bitmap.width,bitmap.height));const canvas=document.createElement("canvas");canvas.width=Math.round(bitmap.width*scale);canvas.height=Math.round(bitmap.height*scale);canvas.getContext("2d")?.drawImage(bitmap,0,0,canvas.width,canvas.height);bitmap.close();logo=canvas.toDataURL("image/webp",.84)}
  async function finish(){saving=true;message="";paymentError="";try{
    const created=await signupTenant({
      name: storeName.trim(),
      unit_name: unitName.trim(),
      email: email.trim(),
      password,
      cnpj_cpf: cnpj.trim() || undefined,
      phone: phone.trim() || undefined,
      plan,
      billing_type: paymentMethod === "pix" ? "PIX" : paymentMethod === "credit_card" ? "CREDIT_CARD" : "BOLETO",
      logo_data_url: logo || undefined,
      primary_color: "#0D5C3A",
    });
    tenantCreatedId = created.tenant_id;
    tenantSlug = created.slug;
    ownerEmail = created.owner_email;
    paymentResult = created.payment ?? null;
    paymentError = created.payment_error ?? "";

    const branding={name:created.name,logo:logo||""};
    localStorage.setItem("commercectrl.client.branding",JSON.stringify(branding));
    localStorage.setItem("commercectrl.client.tenant_id", created.tenant_id);
    localStorage.setItem("commercectrl.client.tenant_slug", created.slug);
    localStorage.setItem("commercectrl.onboarding.profile",JSON.stringify({legalName,cnpj,ie,email,phone,address,unitName,plan,paymentMethod,tenant_id:created.tenant_id,slug:created.slug}));
    if (created.payment) localStorage.setItem("commercectrl.client.payment",JSON.stringify(created.payment));
    window.dispatchEvent(new CustomEvent("commercectrl-branding",{detail:branding}));
    message = created.payment
      ? (created.payment.qr_payload ? "Assinatura criada! Escaneie o QR Code PIX abaixo." : "Assinatura criada! Acesse o link de pagamento para concluir.")
      : "Loja criada em período de teste. Gere a cobrança para ativar a assinatura.";
    step=4;
  }catch(error){message=error instanceof Error?error.message:"Não foi possível criar a loja agora. Verifique a conexão."}finally{saving=false}}
</script>
<main class="onboarding"><aside><a href="/" class="wordmark">_,CTRL</a><div><p>Primeira abertura</p><h1>Vamos abrir sua operação.</h1><span>Você cria a loja, recebe o acesso do administrador e já pode baixar o PDV.</span></div><ol>{#each steps as label,index}<li class:current={step===index} class:done={step>index}><i>{#if step>index}<CheckCircle2 size={15}/>{:else}{index+1}{/if}</i>{label}</li>{/each}</ol><small>Você poderá revisar estes dados depois.</small></aside><section class="onboarding-content"><header><span>Etapa {step+1} de {steps.length}</span><strong>{steps[step]}</strong></header>
  {#if step===0}<div class="form"><div class="intro"><Building2/><h2>Quem é o mercado?</h2><p>Usamos esses dados para identificar a operação, documentos e comunicações do sistema.</p></div><label>Nome fantasia<input class="input" required bind:value={storeName} placeholder="Ex.: Mercadinho Martins"/></label><label>Razão social<input class="input" bind:value={legalName} placeholder="Nome empresarial"/></label><label>CNPJ<input class="input" bind:value={cnpj} inputmode="numeric" placeholder="00.000.000/0001-00"/></label><label>Inscrição estadual<input class="input" bind:value={ie} placeholder="Opcional por enquanto"/></label><label>E-mail do responsável<input class="input" type="email" bind:value={email} placeholder="dono@mercado.com.br"/></label><label>Telefone<input class="input" bind:value={phone} placeholder="(11) 99999-9999"/></label><label>Senha do administrador<input class="input" type="password" bind:value={password} autocomplete="new-password" placeholder="Mínimo 8 caracteres"/></label><label>Confirmar senha<input class="input" type="password" bind:value={passwordConfirm} autocomplete="new-password" placeholder="Repita a senha"/></label>{#if password.length>0&&!passwordValid}<p class="span2 field-error">A senha deve ter entre 8 e 128 caracteres e as duas digitações precisam ser iguais.</p>{/if}</div>
  {:else if step===1}<div class="form"><div class="intro"><MapPin/><h2>Cadastre a primeira unidade</h2><p>Preço, estoque, promoções e telas de TV podem ser administrados por unidade.</p></div><label>Nome da unidade<input class="input" bind:value={unitName}/></label><label class="span2">Endereço completo<input class="input" bind:value={address} placeholder="Rua, número, bairro, cidade e CEP"/></label><div class="callout span2"><ShieldCheck size={19}/><span>Os dados fiscais ficam separados por unidade. Eles podem ser concluídos antes da emissão de NFC-e.</span></div></div>
  {:else if step===2}<div class="form"><div class="intro"><ImageUp/><h2>Deixe o mercado reconhecível</h2><p>A logo aparece de forma elegante no cabeçalho, comprovante e TV de ofertas. Ela é convertida para WebP.</p></div><label class="span2 upload">Logo do mercado<input class="input" type="file" accept="image/png,image/jpeg,image/webp,image/svg+xml" onchange={chooseLogo}/></label>{#if logo}<div class="logo-box span2"><img src={logo} alt="Prévia da logo"/><span>Prévia pronta para sincronização.</span></div>{/if}<div class="callout span2"><PackagePlus size={19}/><span>O próximo passo dentro do sistema será importar ou cadastrar produtos e imagens reais.</span></div></div>
  {:else if step===3}<div class="form"><div class="intro"><ShieldCheck/><h2>Escolha plano e pagamento.</h2><p>O plano define o que você pode usar no sistema. O pagamento define como o Asaas cobrará a assinatura mensal.</p></div><div class="plans span2">{#each plans as option}<button class:selected={plan===option.id} onclick={()=>plan=option.id}><span>{option.name}</span><strong>{option.price}</strong><small>{option.detail}</small>{#if plan===option.id}<i>Selecionado</i>{/if}</button>{/each}</div><div class="payments span2"><h3>Método de pagamento</h3>{#each paymentOptions as opt}<button class:selected={paymentMethod===opt.id} onclick={()=>paymentMethod=opt.id} class="pay-option"><span class="pay-icon" style="width:32px;height:32px;display:inline-block"><svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">{@html opt.icon}</svg></span><div><strong>{opt.name}</strong><small>{opt.detail}</small></div></button>{/each}</div></div>
  {:else}<div class="complete"><CheckCircle2/><h2>Base da loja preparada.</h2><p>{message || "Revise os dados e confirme para salvar a identidade do tenant."}</p>
    {#if tenantCreatedId}
      <div class="payment-panel">
        <h3>Acesso do administrador</h3>
        <div class="payment-info">
          <p><strong>Loja:</strong> {tenantSlug}</p>
          <p><strong>E-mail:</strong> {ownerEmail}</p>
        </div>
        <p class="payment-note">Use a senha que você acabou de definir para entrar no painel. O PDV será vinculado a esta loja após a ativação.</p>
        <button class="copy-btn" onclick={()=>{navigator.clipboard.writeText(`Loja: ${tenantSlug}\nE-mail: ${ownerEmail}`);message="Credenciais copiadas."}}>Copiar credenciais</button>
      </div>
      <div class="payment-panel">
        <div class="payment-info">
          <h3>Pagamento da assinatura</h3>
          <p><strong>Tenant:</strong> {tenantCreatedId}</p>
          <p><strong>Plano:</strong> {plan}</p>
          <p><strong>Status:</strong> {paymentResult?.status || "pendente"}</p>
        </div>
        {#if paymentError}<p class="payment-note" style="color:#a33">{paymentError}</p>{/if}
        {#if paymentResult?.qr_payload}
          <div class="qr-box">
            <QrCode size={160} color="#0D5C3A" />
            <div class="qr-label">Escaneie o QR Code PIX para pagar</div>
            <div class="copy-qr" style="font-size:11px;word-break:break-all;margin-top:6px">{paymentResult.qr_payload}</div>
            <button class="button copy-btn" onclick={()=>{navigator.clipboard.writeText(paymentResult.qr_payload||"");message="Código PIX copiado."}}>Copiar código PIX</button>
          </div>
        {:else if paymentResult?.checkout_url}
          <div class="qr-box">
            <Link2 size={32} color="#0D5C3A" />
            <div class="qr-label">Acesse o link de pagamento</div>
            <a href={paymentResult.checkout_url} target="_blank" rel="noopener" class="button pay-link">Pagar via Asaas</a>
          </div>
        {/if}
        <p class="payment-note">Após a confirmação do pagamento, seu tenant será ativado automaticamente e você poderá baixar o PDV.</p>
      </div>
    {/if}
    {#if desktopDownloadUrl}<a class="button download" href={desktopDownloadUrl}>Baixar o aplicativo para Windows</a>{:else}<div class="download-wait"><strong>Instalador em liberação</strong><span>O administrador da plataforma habilita o download após ativar a assinatura e o primeiro terminal.</span></div>{/if}<button class="back panel-link" disabled={saving} onclick={onDone}>Ir para o painel</button></div>{/if}
  <footer>{#if step>0&&step<4}<button class="back" onclick={()=>step--}><ChevronLeft size={17}/>Voltar</button>{:else}<span></span>{/if}{#if step<3}<button class="next" disabled={step===0&&(!storeName.trim()||!email.includes("@")||!passwordValid)} onclick={()=>step++}>Continuar<ChevronRight size={17}/></button>{:else if step===3}<button class="next" disabled={saving||!email.includes("@")||!passwordValid} onclick={finish}>{saving?"Ativando…":"Ativar e concluir"}<ChevronRight size={17}/></button>{/if}</footer></section></main>
<style>
 .onboarding{min-height:100vh;display:grid;grid-template-columns:minmax(270px,34%) 1fr;background:#f5f4ef;color:#1a2924}.onboarding aside{padding:48px clamp(28px,5vw,80px);background:#153e31;color:#f4f2e8;display:flex;flex-direction:column}.wordmark{font:900 21px "Space Grotesk";letter-spacing:-.11em;color:#dced75;text-decoration:none}.onboarding aside>div{margin-top:clamp(65px,13vh,150px)}.onboarding aside p{color:#b6d1b3;font-size:13px}.onboarding h1{font:700 clamp(36px,4vw,58px)/.94 "Space Grotesk";letter-spacing:-.07em;margin:10px 0 14px}.onboarding aside span{color:#c6d8c3;line-height:1.5}.onboarding ol{padding:0;list-style:none;margin:75px 0 auto;display:grid;gap:20px}.onboarding li{display:flex;align-items:center;gap:12px;color:#99b5a7;font-weight:650}.onboarding li i{width:26px;height:26px;border:1px solid #57796a;border-radius:50%;display:grid;place-items:center;font-style:normal;font-size:12px}.onboarding li.current{color:white}.onboarding li.current i,.onboarding li.done i{background:#dced75;color:#153e31;border-color:#dced75}.onboarding aside small{color:#88a797}.onboarding-content{max-width:800px;width:100%;padding:clamp(32px,8vw,100px);margin:auto}.onboarding-content header{display:flex;justify-content:space-between;color:#668074;font-size:13px}.onboarding-content header strong{color:#153e31}.form{margin-top:72px;display:grid;grid-template-columns:1fr 1fr;gap:20px}.intro,.span2{grid-column:1/-1}.intro :global(svg),.complete :global(svg){width:35px;height:35px;color:#d94c2e}.intro h2,.complete h2{font:700 38px/.98 "Space Grotesk";letter-spacing:-.06em;margin:14px 0 10px}.intro p{max-width:540px;color:#617067;line-height:1.5}.form label{display:grid;gap:7px;font-size:12px;font-weight:750}.callout{display:flex;gap:10px;align-items:center;background:#e6ead8;padding:14px;color:#35543e;font-size:13px;line-height:1.4}.logo-box{display:flex;align-items:center;gap:16px}.logo-box img{width:120px;height:80px;object-fit:contain;background:white;border:1px solid #ccd4ca}.onboarding-content footer{margin-top:70px;display:flex;justify-content:space-between}.next,.back{display:flex;align-items:center;gap:5px;font:700 14px Inter;border:0;padding:12px 17px;cursor:pointer}.next{background:#153e31;color:white}.back{background:transparent;color:#27483a}.complete{margin-top:150px;max-width:530px}.complete :global(svg){width:50px;height:50px;color:#238456}.complete p{color:#617067;line-height:1.5;margin-bottom:25px}@media(max-width:700px){.onboarding{grid-template-columns:1fr}.onboarding aside{padding:28px}.onboarding aside>div{margin-top:40px}.onboarding ol{display:none}.onboarding aside small{display:none}.onboarding-content{padding:38px 24px}.form{margin-top:44px;grid-template-columns:1fr}.intro h2,.complete h2{font-size:32px}}
 .plans{display:grid;gap:10px}.plans button{text-align:left;display:grid;grid-template-columns:1fr auto;gap:6px;padding:19px;background:#fff;border:1px solid #cbd7cf;color:#19352a;cursor:pointer}.plans button.selected{border:2px solid #238456;background:#edf6ec}.plans button span{font-weight:800;font-size:18px}.plans button strong{font-size:15px}.plans button small{grid-column:1/-1;color:#617067}.plans button i{grid-column:2;grid-row:2;color:#238456;font-size:11px;font-style:normal;font-weight:800}.payments{display:flex;flex-direction:column;gap:10px}.payments h3{margin:0 0 8px;font:700 16px/1 "Space Grotesk";color:#19352a}.pay-option{display:flex;align-items:center;gap:12px;padding:14px;background:#fff;border:1px solid #cbd7cf;border-radius:10px;color:#19352a;cursor:pointer;transition:border-color .15s,box-shadow .15s}.pay-option:hover{border-color:#238456}.pay-option.selected{border:2px solid #238456;box-shadow:0 0 0 3px rgba(35,132,86,.12)}.pay-option .pay-icon{flex:none;color:#238456}.pay-option div strong{display:block;font-weight:800;font-size:15px}.pay-option div small{color:#617067;font-size:13px}.download{display:inline-flex;text-decoration:none;margin-bottom:12px}.download-wait{display:grid;gap:5px;padding:15px;border-left:3px solid #d94c2e;background:#fff1e8;margin-bottom:12px}.download-wait span{font-size:13px;color:#617067;line-height:1.45}.panel-link{margin-top:4px}
.payment-panel{margin-top:28px;background:#fff;border:1px solid #cbd7cf;border-radius:12px;padding:20px}
.payment-info{margin-bottom:16px;color:#35543e;font-size:14px}
.payment-info strong{color:#153e31}
.qr-box{text-align:center;margin-bottom:16px}
.qr-label{margin-top:8px;font-weight:700;color:#153e31}
.copy-qr{background:#f4f2e8;border:1px dashed #0D5C3A;border-radius:6px;padding:8px;color:#153e31}
.copy-btn{margin-top:8px;background:#0D5C3A;color:#fff;border:none;border-radius:6px;padding:8px 14px;cursor:pointer;font-weight:700;font-size:13px}
.copy-btn:hover{background:#238456}
.payment-note{font-size:13px;color:#617067;line-height:1.5}
.pay-link{background:#fff;color:#0D5C3A;border:1px solid #0D5C3A;border-radius:6px;padding:10px 16px;text-decoration:none;display:inline-block;font-weight:700;margin-top:8px}
.pay-link:hover{background:#edf6ec}
.field-error{color:#a33;font-size:12px;margin:0;line-height:1.4}
</style>


