import type { Metadata } from "next";
import "../globals.css";
import { ThemeProvider } from "@/components/theme-provider";

export const metadata: Metadata = {
  title: "Display de Promoções - ADEGA_PRO",
  description: "Tela de promoções para exibição em TV.",
};

export default function DisplayLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="pt-BR" suppressHydrationWarning>
      <head>
        <link rel="preconnect" href="https://fonts.googleapis.com" />
        <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
        <link href="https://fonts.googleapis.com/css2?family=Anton&family=Poppins:wght@400;700;900&display=swap" rel="stylesheet" />
      </head>
      <body>
         <ThemeProvider
          attribute="class"
          defaultTheme="dark"
          enableSystem={false}
          disableTransitionOnChange
        >
            {children}
        </ThemeProvider>
      </body>
    </html>
  );
}
