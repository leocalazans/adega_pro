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
import { MoreHorizontal, PlusCircle, CheckCircle, Clock } from "lucide-react";
import { getExpenses } from "@/lib/data";

const getStatusBadgeVariant = (status: string) => {
    return status === 'Pago' ? 'secondary' : 'destructive';
}

const getCategoryBadgeVariant = (category: string) => {
    switch (category) {
        case 'Folha de Pagamento': return 'default';
        case 'Contas Fixas': return 'outline';
        case 'Aluguel': return 'default';
        case 'Fornecedores': return 'secondary';
        case 'Serviços': return 'outline';
        default: return 'outline';
    }
}

export default async function ExpensesPage() {
  const expenses = await getExpenses();

  const totalExpenses = expenses.reduce((acc, expense) => acc + expense.amount, 0);
  const pendingExpenses = expenses.filter(e => e.status === 'Pendente');
  const totalPendingAmount = pendingExpenses.reduce((acc, expense) => acc + expense.amount, 0);

  return (
    <div className="space-y-8">
       <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total de Despesas (Mês)</CardTitle>
            <MoreHorizontal className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">R$ {totalExpenses.toFixed(2)}</div>
            <p className="text-xs text-muted-foreground">+2.5% vs mês passado</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Despesas Pendentes</CardTitle>
            <Clock className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">R$ {totalPendingAmount.toFixed(2)}</div>
            <p className="text-xs text-muted-foreground">{pendingExpenses.length} despesas a vencer</p>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="font-headline">Controle de Despesas</CardTitle>
              <CardDescription>Registre e monitore todas as saídas financeiras.</CardDescription>
            </div>
            <Button>
              <PlusCircle className="mr-2 h-4 w-4" />
              Adicionar Despesa
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Descrição</TableHead>
                <TableHead className="hidden md:table-cell">Categoria</TableHead>
                <TableHead>Valor</TableHead>
                <TableHead className="hidden md:table-cell">Data Venc.</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>
                  <span className="sr-only">Ações</span>
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {expenses.map(expense => (
                  <TableRow key={expense.id}>
                      <TableCell className="font-medium">
                          {expense.description}
                      </TableCell>
                      <TableCell className="hidden md:table-cell">
                          <Badge variant={getCategoryBadgeVariant(expense.category)}>{expense.category}</Badge>
                      </TableCell>
                      <TableCell>R$ {expense.amount.toFixed(2)}</TableCell>
                      <TableCell className="hidden md:table-cell">{expense.dueDate}</TableCell>
                      <TableCell>
                          <Badge variant={getStatusBadgeVariant(expense.status)} className="flex items-center w-fit gap-1">
                              {expense.status === 'Pago' ? <CheckCircle className="h-3 w-3" /> : <Clock className="h-3 w-3" />}
                              {expense.status}
                          </Badge>
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
                              {expense.status === 'Pendente' && (
                                <DropdownMenuItem>Marcar como Paga</DropdownMenuItem>
                              )}
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
    </div>
  );
}
