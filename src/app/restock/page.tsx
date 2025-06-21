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

// Esta lista é um exemplo. Em um app real, seria derivada da lista de produtos,
// filtrando por aqueles que precisam de atenção.
const productsToRestock = [
  { id: 'PROD005', name: "Água Tônica", stock: 3, minStock: 20, recentSales: "Vendas na última semana: 45 unidades.", progress: 15, status: "Crítico" },
  { id: 'PROD007', name: "Saca-rolhas", stock: 2, minStock: 5, recentSales: "Vendas na última semana: 1 unidade.", progress: 40, status: "Crítico" },
  { id: 'PROD001', name: "Vinho Tinto Suave", stock: 8, minStock: 10, recentSales: "Vendas na última semana: 15 unidades.", progress: 80, status: "Baixo" },
  { id: 'PROD008', name: "Cerveja Pilsen Pack 6", stock: 11, minStock: 15, recentSales: "Vendas na última semana: 22 unidades.", progress: 73, status: "Baixo" },
  { id: 'PROD002', name: "Cerveja Artesanal IPA", stock: 40, minStock: 25, recentSales: "Vendas na última semana: 18 unidades.", progress: 160, status: "OK" },
];

export default function RestockPage() {
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
