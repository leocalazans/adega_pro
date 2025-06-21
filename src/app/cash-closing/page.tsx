'use client';

import { useState } from 'react';
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { CreditCard, DollarSign, Smartphone } from "lucide-react";

// Mock data, in a real app this would come from a database based on the logged-in user and sales records
const shiftSummary = {
  employees: [
    { id: 'EMP01', name: 'Ana Silva' },
    { id: 'EMP03', name: 'Carlos Dias' },
    { id: 'EMP05', name: 'Eduardo Lima' },
  ],
  sales: {
    cash: 750.50,
    card: 1230.00,
    pix: 450.75,
    total: 2431.25,
  },
  cashDrawer: {
    initial: 200.00,
  }
};

export default function CashClosingPage() {
    const [countedAmount, setCountedAmount] = useState('');
    const [selectedEmployee, setSelectedEmployee] = useState('');

    const initialCash = shiftSummary.cashDrawer.initial;
    const cashSales = shiftSummary.sales.cash;
    const expectedInDrawer = initialCash + cashSales;
    
    const parsedCountedAmount = parseFloat(countedAmount) || 0;
    const difference = parsedCountedAmount - expectedInDrawer;

    const getDifferenceColor = () => {
        if (difference < 0) return 'text-destructive';
        if (difference > 0) return 'text-green-600';
        return 'text-muted-foreground';
    }

    return (
        <div className="grid gap-8 md:grid-cols-3">
            <div className="md:col-span-2">
                <Card>
                    <CardHeader>
                        <CardTitle className="font-headline">Fechamento de Caixa</CardTitle>
                        <CardDescription>Confira os valores e finalize o turno do caixa.</CardDescription>
                    </CardHeader>
                    <CardContent className="space-y-6">
                        <div className="space-y-2">
                            <Label htmlFor="employee-select">Funcionário do Caixa</Label>
                            <Select value={selectedEmployee} onValueChange={setSelectedEmployee}>
                                <SelectTrigger id="employee-select" className="w-[300px]">
                                    <SelectValue placeholder="Selecione o funcionário" />
                                </SelectTrigger>
                                <SelectContent>
                                    {shiftSummary.employees.map(emp => (
                                        <SelectItem key={emp.id} value={emp.id}>{emp.name}</SelectItem>
                                    ))}
                                </SelectContent>
                            </Select>
                        </div>
                        <Card>
                            <CardHeader>
                                <CardTitle className="text-lg">Resumo das Vendas do Turno</CardTitle>
                            </CardHeader>
                             <CardContent className="space-y-4">
                                <div className="flex justify-between items-center">
                                    <div className="flex items-center gap-2 text-muted-foreground">
                                        <DollarSign className="h-5 w-5" />
                                        <span>Vendas em Dinheiro</span>
                                    </div>
                                    <span className="font-medium">R$ {cashSales.toFixed(2)}</span>
                                </div>
                                <div className="flex justify-between items-center">
                                     <div className="flex items-center gap-2 text-muted-foreground">
                                        <CreditCard className="h-5 w-5" />
                                        <span>Vendas em Cartão</span>
                                    </div>
                                    <span className="font-medium">R$ {shiftSummary.sales.card.toFixed(2)}</span>
                                </div>
                                 <div className="flex justify-between items-center">
                                    <div className="flex items-center gap-2 text-muted-foreground">
                                        <Smartphone className="h-5 w-5" />
                                        <span>Vendas em PIX</span>
                                    </div>
                                    <span className="font-medium">R$ {shiftSummary.sales.pix.toFixed(2)}</span>
                                </div>
                                <Separator />
                                <div className="flex justify-between items-center font-bold text-lg">
                                    <span>Total de Vendas</span>
                                    <span className="text-primary">R$ {shiftSummary.sales.total.toFixed(2)}</span>
                                </div>
                            </CardContent>
                        </Card>
                    </CardContent>
                </Card>
            </div>
            <div className="md:col-span-1">
                <Card>
                    <CardHeader>
                        <CardTitle className="text-lg font-headline">Conferência do Caixa</CardTitle>
                    </CardHeader>
                    <CardContent className="space-y-4">
                        <div className="flex justify-between">
                            <span className="text-muted-foreground">Saldo Inicial (Sangria)</span>
                            <span className="font-medium">R$ {initialCash.toFixed(2)}</span>
                        </div>
                        <div className="flex justify-between">
                            <span className="text-muted-foreground">(+) Vendas em Dinheiro</span>
                            <span className="font-medium">R$ {cashSales.toFixed(2)}</span>
                        </div>
                        <Separator />
                        <div className="flex justify-between font-bold">
                            <span>Total esperado em caixa</span>
                            <span>R$ {expectedInDrawer.toFixed(2)}</span>
                        </div>
                         <Separator />
                        <div className="space-y-2">
                            <Label htmlFor="counted-amount">Valor Contado no Caixa</Label>
                            <Input 
                                id="counted-amount" 
                                type="number" 
                                placeholder="R$ 0,00"
                                value={countedAmount}
                                onChange={(e) => setCountedAmount(e.target.value)}
                             />
                        </div>
                        <div className="flex justify-between text-lg font-bold">
                            <span>Diferença</span>
                            <span className={getDifferenceColor()}>
                                {difference >= 0 ? `+ R$ ${difference.toFixed(2)}` : `- R$ ${Math.abs(difference).toFixed(2)}`}
                            </span>
                        </div>
                    </CardContent>
                    <CardFooter>
                        <Button className="w-full" disabled={!selectedEmployee || !countedAmount}>Confirmar Fechamento</Button>
                    </CardFooter>
                </Card>
            </div>
        </div>
    );
}
