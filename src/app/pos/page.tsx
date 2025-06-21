import { Card, CardContent, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Search, PlusCircle, MinusCircle, Trash2 } from "lucide-react";
import Image from "next/image";

const products = [
    { id: 1, name: "Vinho Tinto Suave", price: 30.00, stock: 15, image: "https://placehold.co/150x150.png", hint: "wine bottle" },
    { id: 2, name: "Cerveja Artesanal IPA", price: 15.00, stock: 40, image: "https://placehold.co/150x150.png", hint: "beer bottle" },
    { id: 3, name: "Whisky 12 Anos", price: 120.00, stock: 8, image: "https://placehold.co/150x150.png", hint: "whiskey bottle" },
    { id: 4, name: "Gin Importado", price: 130.00, stock: 12, image: "https://placehold.co/150x150.png", hint: "gin bottle" },
    { id: 5, name: "Água Tônica", price: 5.00, stock: 50, image: "https://placehold.co/150x150.png", hint: "soda can" },
    { id: 6, name: "Energético", price: 8.00, stock: 35, image: "https://placehold.co/150x150.png", hint: "energy drink" },
    { id: 7, name: "Saca-rolhas", price: 25.00, stock: 10, image: "https://placehold.co/150x150.png", hint: "corkscrew" },
    { id: 8, name: "Cerveja Pilsen Pack 6", price: 22.00, stock: 25, image: "https://placehold.co/150x150.png", hint: "beer pack" },
];

// This would be a client component with state in a real app
const cartItems = [
    { ...products[0], quantity: 2 },
    { ...products[4], quantity: 4 },
    { ...products[1], quantity: 1 },
];

export default function PosPage() {
    const subtotal = cartItems.reduce((acc, item) => acc + item.price * item.quantity, 0);
    const discount = 0;
    const total = subtotal - discount;

    return (
        <div className="grid h-[calc(100vh-8rem)] grid-cols-1 gap-8 md:grid-cols-3">
            {/* Cart Section */}
            <div className="md:col-span-1">
                <Card className="flex h-full flex-col">
                    <CardHeader className="p-4">
                        <CardTitle className="font-headline text-lg">Carrinho</CardTitle>
                    </CardHeader>
                    <CardContent className="flex-1 space-y-4 p-4">
                        {cartItems.map(item => (
                            <div key={item.id} className="flex items-center gap-4">
                                <Image src={item.image} alt={item.name} width={48} height={48} className="h-12 w-12 rounded-md" data-ai-hint={item.hint} />
                                <div className="flex-1">
                                    <p className="font-medium">{item.name}</p>
                                    <p className="text-sm text-muted-foreground">R$ {item.price.toFixed(2)}</p>
                                </div>
                                <div className="flex items-center gap-2">
                                    <Button variant="ghost" size="icon" className="h-6 w-6"><MinusCircle className="h-4 w-4" /></Button>
                                    <span>{item.quantity}</span>
                                    <Button variant="ghost" size="icon" className="h-6 w-6"><PlusCircle className="h-4 w-4" /></Button>
                                    <Button variant="ghost" size="icon" className="h-6 w-6 text-destructive/80 hover:text-destructive"><Trash2 className="h-4 w-4" /></Button>
                                </div>
                            </div>
                        ))}
                    </CardContent>
                    <CardFooter className="flex flex-col gap-4 p-4">
                        <Separator />
                        <div className="w-full space-y-2 text-sm">
                            <div className="flex justify-between">
                                <span>Subtotal</span>
                                <span>R$ {subtotal.toFixed(2)}</span>
                            </div>
                            <div className="flex justify-between">
                                <span>Desconto</span>
                                <span className="text-primary">- R$ {discount.toFixed(2)}</span>
                            </div>
                            <div className="flex justify-between font-bold text-base">
                                <span>Total</span>
                                <span>R$ {total.toFixed(2)}</span>
                            </div>
                        </div>
                        <Button size="lg" className="w-full">Finalizar Venda</Button>
                    </CardFooter>
                </Card>
            </div>
            
            {/* Products Section */}
            <div className="md:col-span-2">
                <Card className="h-full">
                    <CardHeader className="p-4">
                        <div className="relative">
                            <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
                            <Input placeholder="Buscar produto por nome ou código..." className="pl-8" />
                        </div>
                    </CardHeader>
                    <CardContent className="p-4">
                        <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
                            {products.map(product => (
                                <Card key={product.id} className="group cursor-pointer overflow-hidden transition-all hover:shadow-lg">
                                    <CardContent className="flex flex-col items-center p-2 text-center">
                                        <div className="relative w-full aspect-square">
                                            <Image src={product.image} alt={product.name} layout="fill" objectFit="cover" className="rounded-md" data-ai-hint={product.hint} />
                                        </div>
                                        <p className="mt-2 text-sm font-medium">{product.name}</p>
                                        <p className="text-base font-bold text-primary">R$ {product.price.toFixed(2)}</p>
                                    </CardContent>
                                </Card>
                            ))}
                        </div>
                    </CardContent>
                </Card>
            </div>
        </div>
    );
}
