"use client";

import { useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Loader2, Zap, Lightbulb } from "lucide-react";
import { smartRestock, SmartRestockInput, SmartRestockOutput } from "@/ai/flows/smart-restock";

type ProductData = {
  id: string;
  name: string;
  stock: number;
  minStock: number;
  recentSales: string;
};

export default function RestockSuggestionModal({ product }: { product: ProductData }) {
  const [isOpen, setIsOpen] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [suggestion, setSuggestion] = useState<SmartRestockOutput | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleGetSuggestion = async () => {
    setIsLoading(true);
    setError(null);
    setSuggestion(null);

    const input: SmartRestockInput = {
      productId: product.id,
      productName: product.name,
      currentStock: product.stock,
      minStock: product.minStock,
      recentSalesData: product.recentSales,
    };

    try {
      const result = await smartRestock(input);
      setSuggestion(result);
    } catch (e) {
      setError("Falha ao obter sugestão da IA. Tente novamente.");
      console.error(e);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={setIsOpen}>
      <DialogTrigger asChild>
        <Button variant="outline" size="sm">
          <Zap className="mr-2 h-4 w-4" />
          Sugerir Reposição
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-[480px]">
        <DialogHeader>
          <DialogTitle className="font-headline">Sugestão de Reposição IA</DialogTitle>
          <DialogDescription>
            Análise para o produto: <span className="font-semibold text-primary">{product.name}</span>
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4 py-4">
          {!suggestion && !isLoading && (
             <div className="text-center p-6 bg-secondary/50 rounded-lg flex flex-col items-center">
                <Lightbulb className="w-12 h-12 text-primary/80 mb-4"/>
                <p className="text-sm text-muted-foreground">
                    Clique no botão abaixo para que a inteligência artificial analise os dados de venda e sugira a quantidade ideal para reposição.
                </p>
             </div>
          )}
          {isLoading && (
            <div className="flex flex-col items-center justify-center gap-4 p-8">
              <Loader2 className="h-12 w-12 animate-spin text-primary" />
              <p className="text-muted-foreground">Analisando dados...</p>
            </div>
          )}
          {error && (
            <div className="rounded-md border border-destructive/50 bg-destructive/10 p-4 text-center text-sm text-destructive">
              {error}
            </div>
          )}
          {suggestion && (
            <div className="space-y-4">
              <Card className="bg-primary/5 border-primary/20">
                <CardHeader className="p-4">
                  <CardTitle className="text-center text-primary text-4xl font-bold">
                    {suggestion.restockQuantity}
                  </CardTitle>
                  <p className="text-center text-sm font-medium text-primary/80">Unidades para repor</p>
                </CardHeader>
              </Card>
              <Card>
                <CardHeader className="p-4">
                  <CardTitle className="text-base font-headline">Raciocínio da IA</CardTitle>
                </CardHeader>
                <CardContent className="p-4 pt-0">
                  <p className="text-sm text-muted-foreground">{suggestion.reasoning}</p>
                </CardContent>
              </Card>
            </div>
          )}
        </div>
        <DialogFooter>
            <Button variant="outline" onClick={() => setIsOpen(false)}>Fechar</Button>
            <Button onClick={handleGetSuggestion} disabled={isLoading}>
            {isLoading ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Analisando...
              </>
            ) : (
                <>
                 <Zap className="mr-2 h-4 w-4" />
                 {suggestion ? "Analisar Novamente" : "Analisar com IA"}
                </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
