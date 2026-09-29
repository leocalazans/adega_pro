import Link from "next/link";
import { ArrowUpRight, CheckCircle2, Circle, Download, ExternalLink, Github, ServerCog, ShieldCheck, Database } from "lucide-react";

const stages = [
  {
    title: "Supabase",
    icon: Database,
    href: "https://supabase.com/dashboard/project/sqwpkwsjbbkylbdpjrlu",
    items: [
      "Confirme o projeto e mantenha RLS ativo nas tabelas de negócio.",
      "Crie ou valide os buckets públicos product-images e desktop-releases.",
      "Copie a URL do Connection Pooler (porta 6543) para o Render — não a URL direta do banco.",
    ],
  },
  {
    title: "Render",
    icon: ServerCog,
    href: "https://dashboard.render.com/",
    items: [
      "Defina DATABASE_URL com o Connection Pooler do Supabase.",
      "Mantenha PORT e configure SUPERADMIN_EMAIL e SUPERADMIN_PASSWORD.",
      "Faça um redeploy e confirme /health antes de ligar terminais reais.",
    ],
  },
  {
    title: "GitHub",
    icon: Github,
    href: "https://github.com/leocalazans/adega_pro/settings/secrets/actions",
    items: [
      "Desbloqueie Actions se a conta estiver com cobrança pendente.",
      "Cadastre as chaves do updater e as variáveis de produção como Secrets.",
      "Rode Release Windows signed updater para publicar a primeira versão assinada.",
    ],
  },
];

export default function SetupPage() {
  return (
    <main className="min-h-screen bg-[#f4f0e5] px-5 py-8 text-[#193128] sm:px-10 lg:px-16 lg:py-14">
      <header className="mx-auto flex max-w-6xl items-center justify-between border-b border-[#cdd2bc] pb-5">
        <Link href="/" className="font-headline text-2xl font-bold tracking-[-0.12em] text-[#17553a]">_,CTRL</Link>
        <span className="rounded-full border border-[#9eb59e] px-3 py-1 text-xs font-semibold text-[#38624b]">Preparação de produção</span>
      </header>

      <section className="mx-auto grid max-w-6xl gap-10 py-14 lg:grid-cols-[1.1fr_.9fr] lg:items-end">
        <div>
          <p className="mb-4 text-xs font-bold text-[#4c765b]">CHECKLIST DO ADMINISTRADOR</p>
          <h1 className="max-w-3xl font-headline text-5xl font-bold leading-[.91] tracking-[-.07em] sm:text-7xl">Deixe a operação pronta para vender.</h1>
          <p className="mt-7 max-w-2xl text-lg leading-relaxed text-[#52655a]">Esta página não guarda senhas. Ela é o roteiro seguro para ligar banco, backend, release do Windows e download público, nesta ordem.</p>
        </div>
        <div className="border-l-4 border-[#d7e96d] bg-[#17553a] p-6 text-[#f7f5ee] shadow-[12px_12px_0_#d7e96d]">
          <p className="text-xs font-semibold text-[#c9dfcb]">O que já existe</p>
          <h2 className="mt-2 font-headline text-3xl font-bold tracking-[-.05em]">Backend Rust, PDV local e canal de atualização.</h2>
          <p className="mt-4 text-sm leading-relaxed text-[#d6e2d7]">A primeira release assinada transforma o canal em atualização automática. Até lá, use o instalador local apenas para demonstração.</p>
        </div>
      </section>

      <section className="mx-auto max-w-6xl border-y border-[#cdd2bc]">
        {stages.map((stage, index) => {
          const Icon = stage.icon;
          return <article key={stage.title} className="grid gap-6 border-b border-[#cdd2bc] py-8 last:border-b-0 md:grid-cols-[100px_1fr_auto] md:items-start">
            <div className="flex items-center gap-3 text-[#e34d2e]"><span className="font-headline text-2xl">0{index + 1}</span><Icon size={22}/></div>
            <div><h2 className="font-headline text-3xl font-bold tracking-[-.05em]">{stage.title}</h2><ul className="mt-4 grid gap-3 text-sm leading-relaxed text-[#4e6155]">{stage.items.map((item) => <li className="flex gap-3" key={item}><Circle className="mt-1 shrink-0 text-[#69a76f]" size={12}/><span>{item}</span></li>)}</ul></div>
            <a href={stage.href} target="_blank" rel="noreferrer" className="inline-flex items-center justify-center gap-2 border border-[#52725c] px-4 py-2 text-sm font-bold text-[#1a563a] hover:bg-[#e4ecd9]">Abrir {stage.title}<ExternalLink size={15}/></a>
          </article>;
        })}
      </section>

      <section className="mx-auto mt-10 grid max-w-6xl gap-4 md:grid-cols-2">
        <div className="border border-[#b5c5ad] bg-white p-6"><ShieldCheck className="text-[#1d8a55]"/><h2 className="mt-4 font-headline text-2xl font-bold tracking-[-.04em]">Antes de liberar o acesso</h2><p className="mt-2 text-sm leading-relaxed text-[#53665a]">Teste login, licença e sincronização com um terminal de homologação. Não coloque a Service Role do Supabase no frontend nem em variáveis públicas.</p></div>
        <div className="border border-[#b5c5ad] bg-white p-6"><Download className="text-[#1d8a55]"/><h2 className="mt-4 font-headline text-2xl font-bold tracking-[-.04em]">Depois da primeira release</h2><p className="mt-2 text-sm leading-relaxed text-[#53665a]">Publique o instalador e latest.json no bucket desktop-releases. Só então configure o link de download público na Vercel.</p><Link href="/" className="mt-5 inline-flex items-center gap-2 text-sm font-bold text-[#1a563a]">Voltar ao painel <ArrowUpRight size={15}/></Link></div>
      </section>

      <footer className="mx-auto mt-10 flex max-w-6xl items-center gap-2 text-xs text-[#66776a]"><CheckCircle2 size={14} className="text-[#1d8a55]"/>CommerceCTRL · roteiro de implantação local e cloud</footer>
    </main>
  );
}
