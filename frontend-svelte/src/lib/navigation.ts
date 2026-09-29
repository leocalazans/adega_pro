import { LayoutDashboard, ShoppingCart, Package, ClipboardList, FileText, TrendingDown, Truck, UsersRound, Users, Landmark, Tv2, BadgePercent, Settings, Clock3, Tags, BookOpenCheck } from "@lucide/svelte";

export const routes = [
  { href: "/", label: "Dashboard", icon: LayoutDashboard, permission:"reports" },
  { href: "/pos", label: "Frente de Caixa", icon: ShoppingCart, permission:"pos" },
  { href: "/products", label: "Produtos", icon: Package, permission:"catalog" },
  { href: "/restock", label: "Compras IA", icon: ClipboardList, permission:"inventory" },
  { href: "/reports", label: "Relatórios", icon: FileText, permission:"reports" },
  { href: "/expenses", label: "Financeiro", icon: TrendingDown, permission:"finance" },
  { href: "/accounting", label: "Contabilidade", icon: BookOpenCheck, permission:"finance" },
  { href: "/suppliers", label: "Fornecedores", icon: Truck, permission:"suppliers" },
  { href: "/employees", label: "Funcionários", icon: UsersRound, permission:"employees" },
  { href: "/time-clock", label: "Ponto", icon: Clock3, permission:"time" },
  { href: "/labels", label: "Etiquetas", icon: Tags, permission:"labels" },
  { href: "/contacts", label: "Clientes", icon: Users, permission:"customers" },
  { href: "/promotions", label: "Promoções", icon: BadgePercent, permission:"promotions" },
  { href: "/cash-closing", label: "Fechamento de Caixa", icon: Landmark, permission:"cash" },
  { href: "/display", label: "Modo TV", icon: Tv2, permission:"display" },
  { href: "/settings", label: "Configurações", icon: Settings, permission:"settings" },
] as const;
