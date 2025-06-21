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
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { MoreHorizontal, PlusCircle } from "lucide-react";
import Image from "next/image";
import { parse, differenceInDays, isBefore, startOfToday } from 'date-fns';
import { getProducts } from "@/lib/data";

type BadgeVariant = "destructive" | "secondary" | "default" | "outline" | null | undefined;

const getStatusBadgeVariant = (stock: number, minStock: number): {variant: BadgeVariant, text: string} => {
    if (stock <= 0) return { variant: 'destructive', text: 'Sem Estoque' };
    if (stock < minStock) return { variant: 'destructive', text: 'Crítico' };
    if (stock < minStock * 1.2) return { variant: 'default', text: 'Estoque Baixo' };
    return { variant: 'secondary', text: 'Em Estoque' };
}

const getExpiryInfo = (expiryDate?: string): { text: string; variant: BadgeVariant } => {
    if (!expiryDate) {
        return { text: 'N/A', variant: 'secondary' };
    }

    try {
        const date = parse(expiryDate, 'dd/MM/yyyy', new Date());
        const today = startOfToday();

        if (isBefore(date, today)) {
            return { text: 'Vencido', variant: 'destructive' };
        }

        const daysUntilExpiry = differenceInDays(date, today);

        if (daysUntilExpiry <= 30) {
            return { text: 'Próximo', variant: 'default' };
        }

        return { text: 'OK', variant: 'secondary' };
    } catch (e) {
        return { text: 'Inválido', variant: 'destructive' };
    }
};

export default async function ProductsPage() {
  const products = await getProducts();

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="font-headline">Catálogo de Produtos</CardTitle>
            <CardDescription>Gerencie seus produtos, estoque e datas de validade.</CardDescription>
          </div>
          <Button>
            <PlusCircle className="mr-2 h-4 w-4" />
            Adicionar Produto
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="hidden w-[100px] sm:table-cell">
                <span className="sr-only">Imagem</span>
              </TableHead>
              <TableHead>Nome</TableHead>
              <TableHead>Status Estoque</TableHead>
              <TableHead>Vencimento</TableHead>
              <TableHead className="hidden md:table-cell">Preço</TableHead>
              <TableHead className="hidden md:table-cell">Estoque</TableHead>
              <TableHead>
                <span className="sr-only">Ações</span>
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {products.map(product => {
                const expiryInfo = getExpiryInfo(product.expiryDate);
                const stockStatus = getStatusBadgeVariant(product.stock, product.minStock);
                return (
                    <TableRow key={product.id}>
                        <TableCell className="hidden sm:table-cell">
                            <Image
                                alt={product.name}
                                className="aspect-square rounded-md object-cover"
                                height="64"
                                src={product.image}
                                width="64"
                                data-ai-hint={product.hint}
                            />
                        </TableCell>
                        <TableCell className="font-medium">{product.name}</TableCell>
                        <TableCell>
                            <Badge variant={stockStatus.variant}>{stockStatus.text}</Badge>
                        </TableCell>
                        <TableCell>
                           <div className="flex items-center gap-2">
                                <span className={!product.expiryDate ? "text-muted-foreground" : "font-mono text-xs"}>
                                    {product.expiryDate || 'N/A'}
                                </span>
                                {product.expiryDate && (
                                    <Badge variant={expiryInfo.variant}>{expiryInfo.text}</Badge>
                                )}
                            </div>
                        </TableCell>
                        <TableCell className="hidden md:table-cell">R$ {product.price.toFixed(2)}</TableCell>
                        <TableCell className="hidden md:table-cell">{product.stock}</TableCell>
                        <TableCell>
                            <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                                <Button aria-haspopup="true" size="icon" variant="ghost">
                                <MoreHorizontal className="h-4 w-4" />
                                <span className="sr-only">Toggle menu</span>
                                </Button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end">
                                <DropdownMenuLabel>Ações</DropdownMenuLabel>
                                <DropdownMenuItem>Editar</DropdownMenuItem>
                                <DropdownMenuItem>Duplicar</DropdownMenuItem>
                                <DropdownMenuSeparator />
                                <DropdownMenuItem className="text-destructive">Deletar</DropdownMenuItem>
                            </DropdownMenuContent>
                            </DropdownMenu>
                        </TableCell>
                    </TableRow>
                )
            })}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  );
}
