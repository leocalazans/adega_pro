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
  type LucideIcon,
} from "lucide-react";

export type NavItem = {
    href: string;
    label: string;
    icon: LucideIcon;
    roles: ('Admin' | 'Caixa')[];
}

export const allNavItems: NavItem[] = [
  { href: "/", label: "Dashboard", icon: LayoutDashboard, roles: ['Admin'] },
  { href: "/pos", label: "Frente de Caixa", icon: ShoppingCart, roles: ['Admin', 'Caixa'] },
  { href: "/products", label: "Produtos", icon: Package, roles: ['Admin'] },
  { href: "/restock", label: "Compras IA", icon: ClipboardList, roles: ['Admin'] },
  { href: "/reports", label: "Relatórios", icon: FileText, roles: ['Admin'] },
  { href: "/expenses", label: "Despesas", icon: TrendingDown, roles: ['Admin'] },
  { href: "/suppliers", label: "Fornecedores", icon: Truck, roles: ['Admin'] },
  { href: "/employees", label: "Funcionários", icon: UsersRound, roles: ['Admin'] },
  { href: "/contacts", label: "Clientes", icon: Users, roles: ['Admin'] },
  { href: "/cash-closing", label: "Fechamento de Caixa", icon: Landmark, roles: ['Admin', 'Caixa'] },
];

export const getNavItemsForRole = (role: 'Admin' | 'Caixa') => {
    return allNavItems.filter(item => item.roles.includes(role));
}