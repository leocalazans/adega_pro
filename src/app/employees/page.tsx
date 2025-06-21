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
import { MoreHorizontal, PlusCircle, UserCheck, UserX } from "lucide-react";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { getEmployees } from "@/lib/data";

const getStatusBadgeVariant = (status: string) => {
    return status === 'Ativo' ? 'secondary' : 'destructive';
}

const getRoleBadgeVariant = (role: string) => {
    switch (role) {
        case 'Gerente': return 'default';
        case 'Caixa': return 'outline';
        case 'Estoquista': return 'secondary';
        default: return 'outline';
    }
}

export default async function EmployeesPage() {
  const employees = await getEmployees();
  
  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="font-headline">Gestão de Funcionários</CardTitle>
            <CardDescription>Adicione, edite e gerencie os funcionários da sua loja.</CardDescription>
          </div>
          <Button>
            <PlusCircle className="mr-2 h-4 w-4" />
            Adicionar Funcionário
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Funcionário</TableHead>
              <TableHead className="hidden md:table-cell">Cargo</TableHead>
              <TableHead>Status</TableHead>
              <TableHead>
                <span className="sr-only">Ações</span>
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {employees.map(employee => (
                <TableRow key={employee.id}>
                    <TableCell className="font-medium">
                        <div className="flex items-center gap-3">
                            <Avatar className="h-9 w-9">
                                <AvatarImage src={employee.avatar} alt={employee.name} data-ai-hint={employee.hint} />
                                <AvatarFallback>{employee.name.charAt(0)}</AvatarFallback>
                            </Avatar>
                            <div className="grid gap-0.5">
                                <span className="font-medium">{employee.name}</span>
                                <span className="text-xs text-muted-foreground">{`ID: ${employee.id}`}</span>
                            </div>
                        </div>
                    </TableCell>
                    <TableCell className="hidden md:table-cell">
                        <Badge variant={getRoleBadgeVariant(employee.role)}>{employee.role}</Badge>
                    </TableCell>
                    <TableCell>
                        <Badge variant={getStatusBadgeVariant(employee.status)}>{employee.status}</Badge>
                    </TableCell>
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
                            <DropdownMenuItem>Ver Atividade</DropdownMenuItem>
                            <DropdownMenuSeparator />
                            {employee.status === 'Ativo' ? (
                                <DropdownMenuItem className="text-destructive">
                                    <UserX className="mr-2 h-4 w-4" />
                                    Desativar
                                </DropdownMenuItem>
                            ) : (
                                <DropdownMenuItem>
                                    <UserCheck className="mr-2 h-4 w-4" />
                                    Ativar
                                </DropdownMenuItem>
                            )}
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
