"use client";
import { UserNav } from "@/components/user-nav";
import { SidebarTrigger } from "@/components/ui/sidebar";
import { usePathname } from "next/navigation";
import {
  LayoutDashboard,
  ShoppingCart,
  Package,
  FileText,
  Users,
  UsersRound,
  TrendingDown,
  Truck,
  ClipboardList,
  Landmark,
} from "lucide-react";

const navItems = [
  { href: "/", label: "Dashboard", icon: LayoutDashboard },
  { href: "/pos", label: "Frente de Caixa", icon: ShoppingCart },
  { href: "/products", label: "Produtos", icon: Package },
  { href: "/restock", label: "Compras IA", icon: ClipboardList },
  { href: "/reports", label: "Relatórios", icon: FileText },
  { href: "/expenses", label: "Despesas", icon: TrendingDown },
  { href: "/suppliers", label: "Fornecedores", icon: Truck },
  { href: "/employees", label: "Funcionários", icon: UsersRound },
  { href: "/contacts", label: "Clientes", icon: Users },
  { href: "/cash-closing", label: "Fechamento de Caixa", icon: Landmark },
];

export default function Header() {
  const pathname = usePathname();
  const pageTitle = navItems.find((item) => item.href === pathname)?.label || "ADEGA_PRO";
  
  return (
    <header className="sticky top-0 z-10 flex h-16 items-center gap-4 border-b bg-background/80 px-4 backdrop-blur-sm md:px-6">
      <div className="lg:hidden">
        <SidebarTrigger />
      </div>
      <h1 className="text-lg font-semibold md:text-xl font-headline">{pageTitle}</h1>
      <div className="ml-auto flex items-center gap-4">
        <UserNav />
      </div>
    </header>
  );
}
