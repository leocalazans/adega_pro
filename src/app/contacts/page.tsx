import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Users } from "lucide-react";

export default function ContactsPage() {
  return (
    <div className="flex flex-1 items-center justify-center rounded-lg border border-dashed shadow-sm">
      <div className="flex flex-col items-center gap-2 text-center">
        <Users className="h-16 w-16 text-muted-foreground" />
        <h3 className="text-2xl font-bold tracking-tight font-headline">
          Gestão de Contatos
        </h3>
        <p className="text-sm text-muted-foreground">
          Esta funcionalidade está em construção.
        </p>
      </div>
    </div>
  );
}
