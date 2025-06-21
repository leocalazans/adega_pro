"use client";

import { usePathname } from "next/navigation";
import Link from "next/link";
import {
  Sidebar,
  SidebarContent,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarFooter,
} from "@/components/ui/sidebar";
import {
  LayoutDashboard,
  ShoppingCart,
  Package,
  FileText,
  Users,
  Settings,
  Wine,
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

export function AppSidebar() {
  const pathname = usePathname();

  return (
    <Sidebar variant="sidebar" collapsible="icon" className="hidden lg:flex">
      <SidebarHeader className="h-16 items-center justify-center p-4">
        <Link href="/" className="flex items-center gap-2">
          <Wine className="h-8 w-8 text-primary" />
          <span className="font-headline text-xl font-semibold text-primary group-data-[state=collapsed]:hidden">
            ADEGA_PRO
          </span>
        </Link>
      </SidebarHeader>
      <SidebarContent className="flex-1 p-2">
        <SidebarMenu>
          {navItems.map((item) => (
            <SidebarMenuItem key={item.href}>
              <SidebarMenuButton
                asChild
                isActive={pathname === item.href}
                tooltip={{
                  children: item.label,
                }}
              >
                <Link href={item.href}>
                  <item.icon />
                  <span>{item.label}</span>
                </Link>
              </SidebarMenuButton>
            </SidebarMenuItem>
          ))}
        </SidebarMenu>
      </SidebarContent>
      <SidebarFooter className="p-2">
         <SidebarMenu>
          <SidebarMenuItem>
              <SidebarMenuButton
                asChild
                tooltip={{
                  children: "Configurações",
                }}
              >
                <Link href="#">
                  <Settings />
                  <span>Configurações</span>
                </Link>
              </SidebarMenuButton>
            </SidebarMenuItem>
         </SidebarMenu>
      </SidebarFooter>
    </Sidebar>
  );
}
