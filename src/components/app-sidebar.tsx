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
  Wallet,
  FileText,
  Users,
  Settings,
  Wine,
} from "lucide-react";

const navItems = [
  { href: "/", label: "Dashboard", icon: LayoutDashboard },
  { href: "/pos", label: "Frente de Caixa", icon: ShoppingCart },
  { href: "/products", label: "Produtos", icon: Package },
  { href: "/restock", label: "Reposição IA", icon: Wallet },
  { href: "/reports", label: "Relatórios", icon: FileText },
  { href: "/contacts", label: "Contatos", icon: Users },
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
              <Link href={item.href} legacyBehavior passHref>
                <SidebarMenuButton
                  isActive={pathname === item.href}
                  tooltip={{
                    children: item.label,
                  }}
                >
                  <item.icon />
                  <span>{item.label}</span>
                </SidebarMenuButton>
              </Link>
            </SidebarMenuItem>
          ))}
        </SidebarMenu>
      </SidebarContent>
      <SidebarFooter className="p-2">
         <SidebarMenu>
          <SidebarMenuItem>
              <Link href="#" legacyBehavior passHref>
                <SidebarMenuButton
                  tooltip={{
                    children: "Configurações",
                  }}
                >
                  <Settings />
                  <span>Configurações</span>
                </SidebarMenuButton>
              </Link>
            </SidebarMenuItem>
         </SidebarMenu>
      </SidebarFooter>
    </Sidebar>
  );
}
