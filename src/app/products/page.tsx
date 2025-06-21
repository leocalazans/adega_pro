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

const products = [
    { id: 'PROD001', name: "Vinho Tinto Suave", price: 30.00, stock: 8, status: 'Estoque Baixo', image: "https://placehold.co/40x40.png", hint: "wine bottle" },
    { id: 'PROD002', name: "Cerveja Artesanal IPA", price: 15.00, stock: 40, status: 'Em Estoque', image: "https://placehold.co/40x40.png", hint: "beer bottle" },
    { id: 'PROD003', name: "Whisky 12 Anos", price: 120.00, stock: 15, status: 'Em Estoque', image: "https://placehold.co/40x40.png", hint: "whiskey bottle" },
    { id: 'PROD004', name: "Gin Importado", price: 130.00, stock: 12, status: 'Em Estoque', image: "https://placehold.co/40x40.png", hint: "gin bottle" },
    { id: 'PROD005', name: "Água Tônica", price: 5.00, stock: 3, status: 'Crítico', image: "https://placehold.co/40x40.png", hint: "soda can" },
    { id: 'PROD006', name: "Energético", price: 8.00, stock: 50, status: 'Em Estoque', image: "https://placehold.co/40x40.png", hint: "energy drink" },
    { id: 'PROD007', name: "Saca-rolhas", price: 25.00, stock: 2, status: 'Crítico', image: "https://placehold.co/40x40.png", hint: "corkscrew" },
];

const getStatusBadgeVariant = (status: string) => {
    switch (status) {
        case 'Em Estoque': return 'secondary';
        case 'Estoque Baixo': return 'default';
        case 'Crítico': return 'destructive';
        default: return 'outline';
    }
}

export default function ProductsPage() {
  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="font-headline">Catálogo de Produtos</CardTitle>
            <CardDescription>Gerencie seus produtos e estoque.</CardDescription>
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
              <TableHead>Status</TableHead>
              <TableHead className="hidden md:table-cell">Preço</TableHead>
              <TableHead className="hidden md:table-cell">Estoque</TableHead>
              <TableHead>
                <span className="sr-only">Ações</span>
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {products.map(product => (
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
            ))}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  );
}
