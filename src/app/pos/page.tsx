'use client';

import { useState, useEffect, useRef, useCallback } from 'react';
import { Card, CardContent, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Search, PlusCircle, MinusCircle, Trash2, Camera, ShoppingCart, CreditCard, DollarSign, Wallet, Smartphone } from "lucide-react";
import Image from "next/image";
import { useToast } from "@/hooks/use-toast";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { Label } from '@/components/ui/label';

// In a real app, this would come from an API
const productCatalog = [
    { id: 1, name: "Vinho Tinto Suave", price: 30.00, stock: 15, image: "https://placehold.co/150x150.png", hint: "wine bottle", barcode: "78900001" },
    { id: 2, name: "Cerveja Artesanal IPA", price: 15.00, stock: 40, image: "https://placehold.co/150x150.png", hint: "beer bottle", barcode: "78900002" },
    { id: 3, name: "Whisky 12 Anos", price: 120.00, stock: 8, image: "https://placehold.co/150x150.png", hint: "whiskey bottle", barcode: "78900003" },
    { id: 4, name: "Gin Importado", price: 130.00, stock: 12, image: "https://placehold.co/150x150.png", hint: "gin bottle", barcode: "78900004" },
    { id: 5, name: "Água Tônica", price: 5.00, stock: 50, image: "https://placehold.co/150x150.png", hint: "soda can", barcode: "78900005" },
    { id: 6, name: "Energético", price: 8.00, stock: 35, image: "https://placehold.co/150x150.png", hint: "energy drink", barcode: "78900006" },
    { id: 7, name: "Saca-rolhas", price: 25.00, stock: 10, image: "https://placehold.co/150x150.png", hint: "corkscrew", barcode: "78900007" },
    { id: 8, name: "Cerveja Pilsen Pack 6", price: 22.00, stock: 25, image: "https://placehold.co/150x150.png", hint: "beer pack", barcode: "78900008" },
];

type Product = typeof productCatalog[0];
type CartItem = Product & { quantity: number };

export default function PosPage() {
    const { toast } = useToast();
    const [cartItems, setCartItems] = useState<CartItem[]>([]);
    const [searchTerm, setSearchTerm] = useState("");
    const [isCameraOpen, setIsCameraOpen] = useState(false);
    const [hasCameraPermission, setHasCameraPermission] = useState<boolean | null>(null);
    const [isPaymentDialogOpen, setIsPaymentDialogOpen] = useState(false);
    const [paymentMethod, setPaymentMethod] = useState("credit_card");
    const [amountPaid, setAmountPaid] = useState("");

    const videoRef = useRef<HTMLVideoElement>(null);
    const searchInputRef = useRef<HTMLInputElement>(null);

    const handleAddToCart = useCallback((product: Product) => {
        setCartItems(prevItems => {
            const existingItem = prevItems.find(item => item.id === product.id);
            if (existingItem) {
                return prevItems.map(item =>
                    item.id === product.id ? { ...item, quantity: item.quantity + 1 } : item
                );
            }
            return [...prevItems, { ...product, quantity: 1 }];
        });
    }, []);

    const handleUpdateQuantity = (productId: number, newQuantity: number) => {
        if (newQuantity <= 0) {
            handleRemoveFromCart(productId);
        } else {
            setCartItems(prevItems =>
                prevItems.map(item =>
                    item.id === productId ? { ...item, quantity: newQuantity } : item
                )
            );
        }
    };
    
    const handleRemoveFromCart = (productId: number) => {
        setCartItems(prevItems => prevItems.filter(item => item.id !== productId));
    };

    const subtotal = cartItems.reduce((acc, item) => acc + item.price * item.quantity, 0);
    const discount = 0; // Placeholder for discount logic
    const total = subtotal - discount;
    const change = paymentMethod === 'cash' && amountPaid ? parseFloat(amountPaid) - total : 0;

    const handleFinalizeSale = useCallback(() => {
        if (cartItems.length === 0) {
            toast({
                variant: "destructive",
                title: "Carrinho Vazio",
                description: "Adicione produtos ao carrinho antes de finalizar a venda.",
            });
            return;
        }
        setIsPaymentDialogOpen(true);
    }, [cartItems, toast]);

    const handleConfirmPayment = () => {
        const saleTotal = total.toFixed(2);
        let toastDescription = `Total da venda: R$ ${saleTotal}.`;
        
        if (paymentMethod === 'cash') {
            if (!amountPaid || parseFloat(amountPaid) < total) {
                toast({
                    variant: "destructive",
                    title: "Valor Insuficiente",
                    description: "O valor pago deve ser maior ou igual ao total da venda.",
                });
                return;
            }
            toastDescription += ` Troco: R$ ${change.toFixed(2)}.`;
        }

        toast({
            title: "Venda Finalizada com Sucesso!",
            description: toastDescription,
        });

        // Reset state
        setCartItems([]);
        setAmountPaid("");
        setPaymentMethod("credit_card");
        setIsPaymentDialogOpen(false);
    };
    
    const filteredProducts = productCatalog.filter(p =>
        p.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
        p.barcode?.includes(searchTerm)
    );

    useEffect(() => {
        if (!isCameraOpen) {
            if (videoRef.current?.srcObject) {
                const stream = videoRef.current.srcObject as MediaStream;
                stream.getTracks().forEach(track => track.stop());
                videoRef.current.srcObject = null;
            }
            return;
        }
    
        const getCameraPermission = async () => {
          try {
            const stream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: "environment" } });
            setHasCameraPermission(true);
    
            if (videoRef.current) {
              videoRef.current.srcObject = stream;
            }
          } catch (error) {
            console.error('Error accessing camera:', error);
            setHasCameraPermission(false);
            toast({
              variant: 'destructive',
              title: 'Acesso à câmera negado',
              description: 'Por favor, habilite a permissão da câmera nas configurações do seu navegador.',
            });
          }
        };
    
        getCameraPermission();

        return () => {
            if (videoRef.current?.srcObject) {
                const stream = videoRef.current.srcObject as MediaStream;
                stream.getTracks().forEach(track => track.stop());
            }
        }
      }, [isCameraOpen, toast]);

    // Keyboard shortcuts
    useEffect(() => {
        const handleKeyDown = (event: KeyboardEvent) => {
            const target = event.target as HTMLElement;
            if ( (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA') && event.key !== 'Escape' ) {
                return;
            }

            switch (event.key) {
                case 'F2':
                    event.preventDefault();
                    searchInputRef.current?.focus();
                    setSearchTerm('');
                    break;
                case 'F4':
                    event.preventDefault();
                    handleFinalizeSale();
                    break;
                case 'F8':
                    event.preventDefault();
                    setIsCameraOpen(true);
                    break;
                case 'Escape':
                    if (isCameraOpen) setIsCameraOpen(false);
                    if (isPaymentDialogOpen) setIsPaymentDialogOpen(false);
                    break;
            }
        };

        window.addEventListener('keydown', handleKeyDown);

        return () => {
            window.removeEventListener('keydown', handleKeyDown);
        };
    }, [handleFinalizeSale, isCameraOpen, isPaymentDialogOpen]);

      const simulateScan = () => {
        const randomProduct = productCatalog[Math.floor(Math.random() * productCatalog.length)];
        handleAddToCart(randomProduct);
        setIsCameraOpen(false);
        toast({
            title: "Produto Escaneado!",
            description: `${randomProduct.name} foi adicionado ao carrinho.`,
        });
      };

    return (
        <div className="grid h-[calc(100vh-8rem)] grid-cols-1 gap-4 md:grid-cols-3">
            {/* Cart Section */}
            <div className="md:col-span-1">
                <Card className="flex h-full flex-col">
                    <CardHeader className="p-4">
                        <CardTitle className="font-headline text-lg">Carrinho</CardTitle>
                    </CardHeader>
                    <CardContent className="flex-1 space-y-3 p-4 overflow-y-auto">
                        {cartItems.length === 0 ? (
                             <div className="flex flex-col items-center justify-center h-full text-center text-muted-foreground">
                                <ShoppingCart className="h-12 w-12 mb-4" />
                                <p>Seu carrinho está vazio.</p>
                                <p className="text-xs">Adicione produtos da lista ao lado.</p>
                            </div>
                        ) : (
                            cartItems.map(item => (
                            <div key={item.id} className="flex items-center gap-3">
                                <Image src={item.image} alt={item.name} width={48} height={48} className="h-12 w-12 rounded-md object-cover" data-ai-hint={item.hint} />
                                <div className="flex-1">
                                    <p className="font-medium text-sm leading-tight">{item.name}</p>
                                    <p className="text-xs text-muted-foreground">R$ {item.price.toFixed(2)}</p>
                                </div>
                                <div className="flex items-center gap-1">
                                    <Button variant="ghost" size="icon" className="h-6 w-6" onClick={() => handleUpdateQuantity(item.id, item.quantity - 1)}><MinusCircle className="h-4 w-4" /></Button>
                                    <span className="w-6 text-center text-sm">{item.quantity}</span>
                                    <Button variant="ghost" size="icon" className="h-6 w-6" onClick={() => handleUpdateQuantity(item.id, item.quantity + 1)}><PlusCircle className="h-4 w-4" /></Button>
                                    <Button variant="ghost" size="icon" className="h-6 w-6 text-destructive/80 hover:text-destructive" onClick={() => handleRemoveFromCart(item.id)}><Trash2 className="h-4 w-4" /></Button>
                                </div>
                            </div>
                        )))}
                    </CardContent>
                    <CardFooter className="flex flex-col gap-4 p-4 border-t">
                        <div className="w-full space-y-2 text-sm">
                            <div className="flex justify-between">
                                <span>Subtotal</span>
                                <span>R$ {subtotal.toFixed(2)}</span>
                            </div>
                            <div className="flex justify-between">
                                <span>Desconto</span>
                                <span className="text-primary">- R$ {discount.toFixed(2)}</span>
                            </div>
                            <Separator className="my-1"/>
                            <div className="flex justify-between font-bold text-base">
                                <span>Total</span>
                                <span>R$ {total.toFixed(2)}</span>
                            </div>
                        </div>
                        <Button size="lg" className="w-full font-bold" onClick={handleFinalizeSale}>Finalizar Venda</Button>
                    </CardFooter>
                </Card>
            </div>
            
            {/* Products Section */}
            <div className="md:col-span-2">
                <Card className="h-full flex flex-col">
                    <CardHeader className="p-4">
                        <div className="flex gap-2">
                            <div className="relative flex-1">
                                <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
                                <Input 
                                    ref={searchInputRef}
                                    placeholder="Buscar produto por nome ou código... (F2)" 
                                    className="pl-8" 
                                    value={searchTerm}
                                    onChange={(e) => setSearchTerm(e.target.value)}
                                />
                            </div>
                            <Dialog open={isCameraOpen} onOpenChange={setIsCameraOpen}>
                                <DialogTrigger asChild>
                                    <Button variant="outline" className="shrink-0">
                                        <Camera className="mr-2 h-4 w-4" />
                                        Escanear (F8)
                                    </Button>
                                </DialogTrigger>
                                <DialogContent className="sm:max-w-md">
                                    <DialogHeader>
                                        <DialogTitle>Escanear Código de Barras</DialogTitle>
                                        <DialogDescription>Aponte a câmera para o código de barras do produto.</DialogDescription>
                                    </DialogHeader>
                                    <div className="relative mt-4">
                                        <video ref={videoRef} className="w-full aspect-video rounded-md bg-muted" autoPlay muted playsInline />
                                        <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
                                            <div className="w-4/5 h-1/3 border-2 border-primary/50 rounded-lg" />
                                        </div>
                                    </div>
                                    {hasCameraPermission === false && (
                                        <Alert variant="destructive" className="mt-4">
                                            <AlertTitle>Acesso à Câmera Necessário</AlertTitle>
                                            <AlertDescription>
                                                Por favor, permita o acesso à câmera para usar esta funcionalidade.
                                            </AlertDescription>
                                        </Alert>
                                    )}
                                    <DialogFooter className="mt-4">
                                        <Button type="button" variant="secondary" onClick={() => setIsCameraOpen(false)}>Cancelar</Button>
                                        <Button type="button" onClick={simulateScan} disabled={!hasCameraPermission}>
                                            Simular Escaneamento
                                        </Button>
                                    </DialogFooter>
                                </DialogContent>
                            </Dialog>
                        </div>
                    </CardHeader>
                    <CardContent className="p-4 flex-1 overflow-y-auto">
                         <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
                            {filteredProducts.map(product => (
                                <Card key={product.id} className="group cursor-pointer overflow-hidden transition-all hover:shadow-lg hover:-translate-y-1" onClick={() => handleAddToCart(product)}>
                                    <CardContent className="flex flex-col items-center p-2 text-center">
                                        <div className="relative w-full aspect-square">
                                            <Image src={product.image} alt={product.name} fill className="rounded-md object-cover" data-ai-hint={product.hint} />
                                        </div>
                                        <p className="mt-2 text-xs font-medium h-8 flex items-center justify-center">{product.name}</p>
                                        <p className="text-sm font-bold text-primary">R$ {product.price.toFixed(2)}</p>
                                    </CardContent>
                                </Card>
                            ))}
                        </div>
                        {filteredProducts.length === 0 && (
                            <div className="flex flex-col items-center justify-center h-full text-center text-muted-foreground">
                               <Search className="h-12 w-12 mb-4" />
                               <p className="font-semibold">Nenhum produto encontrado</p>
                               <p className="text-xs">Tente um termo de busca diferente.</p>
                           </div>
                        )}
                    </CardContent>
                    <CardFooter className="p-3 border-t">
                        <div className="flex items-center flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
                            <span className="font-semibold">Atalhos:</span>
                            <div className="flex items-center gap-1.5">
                                <kbd className="pointer-events-none inline-flex h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium text-foreground">F2</kbd>
                                <span>Buscar</span>
                            </div>
                            <div className="flex items-center gap-1.5">
                                <kbd className="pointer-events-none inline-flex h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium text-foreground">F4</kbd>
                                <span>Finalizar</span>
                            </div>
                            <div className="flex items-center gap-1.5">
                                <kbd className="pointer-events-none inline-flex h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium text-foreground">F8</kbd>
                                <span>Escanear</span>
                            </div>
                             <div className="flex items-center gap-1.5">
                                <kbd className="pointer-events-none inline-flex h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium text-foreground">Esc</kbd>
                                <span>Sair do Foco/Fechar</span>
                            </div>
                        </div>
                    </CardFooter>
                </Card>
            </div>
             <Dialog open={isPaymentDialogOpen} onOpenChange={setIsPaymentDialogOpen}>
                <DialogContent>
                    <DialogHeader>
                        <DialogTitle className="font-headline">Finalizar Venda</DialogTitle>
                        <DialogDescription>Selecione a forma de pagamento.</DialogDescription>
                    </DialogHeader>
                    <div className="grid gap-6 py-4">
                        <div className="flex items-center justify-between text-2xl font-bold">
                            <span>Total:</span>
                            <span className="text-primary">R$ {total.toFixed(2)}</span>
                        </div>
                        <RadioGroup value={paymentMethod} onValueChange={setPaymentMethod}>
                            <div className="grid grid-cols-2 gap-4">
                                <Label htmlFor="payment_cash" className="flex flex-col items-center justify-center gap-2 rounded-md border p-4 hover:bg-accent hover:text-accent-foreground peer-data-[state=checked]:border-primary [&:has([data-state=checked])]:border-primary">
                                    <RadioGroupItem value="cash" id="payment_cash" className="sr-only"/>
                                    <DollarSign className="h-8 w-8" />
                                    Dinheiro
                                </Label>
                                <Label htmlFor="payment_card" className="flex flex-col items-center justify-center gap-2 rounded-md border p-4 hover:bg-accent hover:text-accent-foreground peer-data-[state=checked]:border-primary [&:has([data-state=checked])]:border-primary">
                                    <RadioGroupItem value="credit_card" id="payment_card" className="sr-only"/>
                                    <CreditCard className="h-8 w-8" />
                                    Cartão
                                </Label>
                                <Label htmlFor="payment_pix" className="flex flex-col items-center justify-center gap-2 rounded-md border p-4 hover:bg-accent hover:text-accent-foreground peer-data-[state=checked]:border-primary [&:has([data-state=checked])]:border-primary">
                                    <RadioGroupItem value="pix" id="payment_pix" className="sr-only"/>
                                    <Smartphone className="h-8 w-8" />
                                    Pix
                                </Label>
                                <Label htmlFor="payment_other" className="flex flex-col items-center justify-center gap-2 rounded-md border p-4 hover:bg-accent hover:text-accent-foreground peer-data-[state=checked]:border-primary [&:has([data-state=checked])]:border-primary">
                                    <RadioGroupItem value="other" id="payment_other" className="sr-only"/>
                                    <Wallet className="h-8 w-8" />
                                    Outro
                                </Label>
                            </div>
                        </RadioGroup>
                        {paymentMethod === 'cash' && (
                            <div className="grid gap-2">
                                <Label htmlFor="amount-paid">Valor Entregue</Label>
                                <Input 
                                    id="amount-paid" 
                                    type="number" 
                                    placeholder="R$ 0,00"
                                    value={amountPaid}
                                    onChange={(e) => setAmountPaid(e.target.value)}
                                />
                                {change > 0 && (
                                    <div className="text-right font-medium text-lg">
                                        Troco: <span className="text-primary">R$ {change.toFixed(2)}</span>
                                    </div>
                                )}
                            </div>
                        )}
                    </div>
                    <DialogFooter>
                        <Button variant="outline" onClick={() => setIsPaymentDialogOpen(false)}>Cancelar</Button>
                        <Button onClick={handleConfirmPayment}>Confirmar Pagamento</Button>
                    </DialogFooter>
                </DialogContent>
            </Dialog>
        </div>
    );
}
