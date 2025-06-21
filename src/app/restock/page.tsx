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
import RestockSuggestionModal from "@/components/restock/restock-suggestion-modal";
import { getProducts } from "@/lib/data";
import type { Product } from "@/lib/types";

// Em um app real, os dados de vendas recentes viriam do banco de dados.
// Aqui estamos adicionando um mock para a demonstração.
const recentSalesData: Record<string, string> = {
  'PROD005': "Vendas na última semana: 45 unidades.",
  'PROD007': "Vendas na última semana: 1 unidade.",
  'PROD001': "Vendas na última semana: 15 unidades.",
  'PROD008': "Vendas na última semana: 22 unidades.",
  'PROD002': "Vendas na última semana: 18 unidades.",
};

type ProductToRestock = Product & {
  recentSales: string;
  progress: number;
  status: 'Crítico' | 'Baixo' | 'OK';
}

export default async function RestockPage() {
  const allProducts = await getProducts();
  
  const productsToRestock: ProductToRestock[] = allProducts
    .map(p => {
      const stockPercentage = (p.stock / p.minStock) * 100;
      let status: ProductToRestock['status'] = 'OK';
      if (stockPercentage < 100) status = 'Baixo';
      if (stockPercentage < 50) status = 'Crítico';

      return {
        ...p,
        recentSales: recentSalesData[p.id] || "Sem dados de vendas recentes.",
        progress: stockPercentage,
        status,
      }
    })
    .sort((a, b) => a.progress - b.progress); // Sort by lowest stock first

  return (
    <Card>
      <CardHeader>
        <CardTitle className="font-headline">Reposição Inteligente de Estoque</CardTitle>
        <CardDescription>
          Analise quais produtos precisam de reposição e use a IA para obter
          sugestões de compra inteligentes, evitando perdas e garantindo que você nunca deixe de vender.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-[40%]">Produto</TableHead>
              <TableHead>Status do Estoque</TableHead>
              <TableHead>Estoque Atual / Mínimo</TableHead>
              <TableHead className="text-right">Ação</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {productsToRestock.map((product) => (
              <TableRow key={product.id}>
                <TableCell className="font-medium">{product.name}</TableCell>
                <TableCell>
                  <div className="flex items-center gap-2">
                    <Progress value={product.progress > 100 ? 100 : product.progress} className="h-2 w-24" />
                    <Badge variant={product.status === "Crítico" ? "destructive" : product.status === "Baixo" ? "default" : "secondary"}>
                      {product.status}
                    </Badge>
                  </div>
                </TableCell>
                <TableCell>{`${product.stock} / ${product.minStock}`}</TableCell>
                <TableCell className="text-right">
                  <RestockSuggestionModal product={product} />
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  );
}
