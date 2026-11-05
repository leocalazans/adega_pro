import { LayoutDashboard, FileText, BadgePercent, Tv2, ShieldCheck, Store, UserRound, Mail } from "@lucide/svelte";

export const routes = [
  { href: "/", label: "Dashboard", icon: LayoutDashboard, permission: "reports" },
  { href: "/reports", label: "Relatórios", icon: FileText, permission: "reports" },
  { href: "/promotions", label: "Promoções", icon: BadgePercent, permission: "promotions" },
  { href: "/display", label: "Modo TV", icon: Tv2, permission: "display" },
] as const;

export const publicRoutes = [
  { href: "/login", label: "Acessar", icon: ShieldCheck },
  { href: "/superadmin", label: "SuperAdmin", icon: Mail },
  { href: "/onboarding", label: "Onboarding", icon: UserRound },
  { href: "/download", label: "Download", icon: Store },
] as const;
