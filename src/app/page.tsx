import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import { DollarSign, Package, ShoppingCart, UserPlus } from "lucide-react";
import SalesChart from "@/components/dashboard/sales-chart";

const stats = [
  {
    title: "Vendas Hoje",
    value: "R$ 1,250.00",
    change: "+15.2% vs ontem",
    icon: DollarSign,
  },
  {
    title: "Itens Vendidos",
    value: "82",
    change: "+8.1% vs ontem",
    icon: ShoppingCart,
  },
  {
    title: "Novos Clientes",
    value: "4",
    change: "+2 vs ontem",
    icon: UserPlus,
  },
  {
    title: "Estoque Baixo",
    value: "12 Itens",
    change: "3 precisam de atenção",
    icon: Package,
  },
];

const topProducts = [
  { name: "Vinho Tinto Suave", sales: 120, revenue: "R$ 3,600.00" },
  { name: "Cerveja Artesanal IPA", sales: 98, revenue: "R$ 1,470.00" },
  { name: "Whisky 12 Anos", sales: 45, revenue: "R$ 5,400.00" },
  { name: "Gin Importado", sales: 32, revenue: "R$ 4,160.00" },
  { name: "Água Tônica", sales: 210, revenue: "R$ 1,050.00" },
];

const lowStockProducts = [
  { name: "Vinho Tinto Suave", stock: 8, minStock: 10, progress: 80 },
  { name: "Energético", stock: 5, minStock: 12, progress: 41 },
  { name: "Saca-rolhas", stock: 3, minStock: 5, progress: 60 },
  { name: "Cerveja Pilsen Pack 6", stock: 11, minStock: 15, progress: 73 },
];

export default function DashboardPage() {
  return (
    <div className="flex flex-col gap-8">
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        {stats.map((stat, index) => (
          <Card key={index}>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">{stat.title}</CardTitle>
              <stat.icon className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">{stat.value}</div>
              <p className="text-xs text-muted-foreground">{stat.change}</p>
            </CardContent>
          </Card>
        ))}
      </div>

      <div className="grid grid-cols-1 gap-8 lg:grid-cols-5">
        <Card className="lg:col-span-3">
          <CardHeader>
            <CardTitle className="font-headline">Visão Geral de Vendas</CardTitle>
            <CardDescription>Últimos 7 dias</CardDescription>
          </CardHeader>
          <CardContent>
            <SalesChart />
          </CardContent>
        </Card>

        <Card className="lg:col-span-2">
          <CardHeader>
            <CardTitle className="font-headline">Produtos Mais Vendidos</CardTitle>
            <CardDescription>Ranking deste mês.</CardDescription>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Produto</TableHead>
                  <TableHead className="text-right">Receita</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {topProducts.map((product) => (
                  <TableRow key={product.name}>
                    <TableCell className="font-medium">{product.name}</TableCell>
                    <TableCell className="text-right">{product.revenue}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      </div>
      
      <Card>
        <CardHeader>
          <CardTitle className="font-headline">Alerta de Estoque Baixo</CardTitle>
          <CardDescription>Produtos que precisam de reposição urgente.</CardDescription>
        </CardHeader>
        <CardContent>
        <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-[40%]">Produto</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Estoque Atual</TableHead>
                <TableHead className="text-right">Estoque Mínimo</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {lowStockProducts.map((product) => (
                <TableRow key={product.name}>
                  <TableCell className="font-medium">{product.name}</TableCell>
                  <TableCell>
                    <div className="flex items-center gap-2">
                      <Progress value={product.progress} className="h-2 w-24" />
                      <Badge variant={product.stock < product.minStock ? "destructive" : "secondary"}>
                        {product.stock < product.minStock ? "Crítico" : "Atenção"}
                      </Badge>
                    </div>
                  </TableCell>
                  <TableCell>{product.stock}</TableCell>
                  <TableCell className="text-right">{product.minStock}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </div>
  );
}
