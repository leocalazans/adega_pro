"use client";

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

const products = [
    { id: 'PROD001', name: "Vinho Tinto Suave", price: 30.00, stock: 8, minStock: 10, status: 'Estoque Baixo', image: "https://placehold.co/64x64.png", hint: "wine bottle", expiryDate: '15/12/2025' },
    { id: 'PROD002', name: "Cerveja Artesanal IPA", price: 15.00, stock: 40, minStock: 25, status: 'Em Estoque', image: "https://placehold.co/64x64.png", hint: "beer bottle", expiryDate: '20/08/2024' },
    { id: 'PROD003', name: "Whisky 12 Anos", price: 120.00, stock: 15, minStock: 5, status: 'Em Estoque', image: "https://placehold.co/64x64.png", hint: "whiskey bottle" },
    { id: 'PROD004', name: "Gin Importado", price: 130.00, stock: 12, minStock: 10, status: 'Em Estoque', image: "https://placehold.co/64x64.png", hint: "gin bottle" },
    { id: 'PROD005', name: "Água Tônica", price: 5.00, stock: 3, minStock: 20, status: 'Crítico', image: "https://placehold.co/64x64.png", hint: "soda can", expiryDate: '30/08/2024' },
    { id: 'PROD006', name: "Energético", price: 8.00, stock: 50, minStock: 20, status: 'Em Estoque', image: "https://placehold.co/64x64.png", hint: "energy drink", expiryDate: '30/07/2024' },
    { id: 'PROD007', name: "Saca-rolhas", price: 25.00, stock: 2, minStock: 5, status: 'Crítico', image: "https://placehold.co/64x64.png", hint: "corkscrew" },
    { id: 'PROD008', name: "Cerveja Pilsen Pack 6", price: 22.00, stock: 11, minStock: 15, status: 'Estoque Baixo', image: "https://placehold.co/64x64.png", hint: "beer pack" },
    { id: 'PROD009', name: "Salgadinho de Queijo", price: 7.50, stock: 25, minStock: 15, status: 'Em Estoque', image: "https://placehold.co/64x64.png", hint: "snack bag", expiryDate: '01/07/2024' },
    { id: 'PROD010', name: "Maço de Cigarros", price: 12.00, stock: 100, minStock: 30, status: 'Em Estoque', image: "https://placehold.co/64x64.png", hint: "cigarette pack" },
    { id: 'PROD011', name: "Isqueiro", price: 4.00, stock: 30, minStock: 10, status: 'Em Estoque', image: "https://placehold.co/64x64.png", hint: "lighter" },
];

type BadgeVariant = "destructive" | "secondary" | "default" | "outline" | null | undefined;

const getStatusBadgeVariant = (status: string): BadgeVariant => {
    switch (status) {
        case 'Em Estoque': return 'secondary';
        case 'Estoque Baixo': return 'default';
        case 'Crítico': return 'destructive';
        default: return 'outline';
    }
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


export default function ProductsPage() {
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
                            <Badge variant={getStatusBadgeVariant(product.status)}>{product.status}</Badge>
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
