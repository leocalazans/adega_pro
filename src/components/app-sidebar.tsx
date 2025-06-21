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
  Settings,
  Wine,
} from "lucide-react";
import { getNavItemsForRole } from "@/lib/nav-config";


// In a real app, this would come from an authentication context.
// Change to 'Caixa' to test the cashier view.
const userRole: 'Admin' | 'Caixa' = 'Admin';


export function AppSidebar() {
  const pathname = usePathname();
  const navItems = getNavItemsForRole(userRole);

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
