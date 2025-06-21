"use client";

import { useState, useEffect } from "react";
import { Bar, BarChart, ResponsiveContainer, XAxis, YAxis, Tooltip } from "recharts";
import {
  ChartContainer,
  ChartTooltipContent,
} from "@/components/ui/chart";
import { useTheme } from "next-themes";
import { Skeleton } from "@/components/ui/skeleton";

const generateChartData = () => [
  { date: "Seg", sales: Math.floor(Math.random() * 2000) + 500 },
  { date: "Ter", sales: Math.floor(Math.random() * 2000) + 500 },
  { date: "Qua", sales: Math.floor(Math.random() * 2000) + 500 },
  { date: "Qui", sales: Math.floor(Math.random() * 2000) + 500 },
  { date: "Sex", sales: Math.floor(Math.random() * 2000) + 500 },
  { date: "Sáb", sales: Math.floor(Math.random() * 3000) + 1000 },
  { date: "Dom", sales: Math.floor(Math.random() * 3000) + 800 },
];

export default function SalesChart() {
    const { theme } = useTheme();
    const isDarkMode = theme === 'dark';
    const [chartData, setChartData] = useState<any[] | null>(null);

    useEffect(() => {
        setChartData(generateChartData());
    }, []);

  const chartConfig = {
    sales: {
      label: "Vendas",
      color: isDarkMode ? "hsl(var(--primary))" : "hsl(var(--primary))",
    },
  };
    
  if (!chartData) {
    return <Skeleton className="h-64 w-full" />;
  }

  return (
    <ChartContainer config={chartConfig} className="h-64 w-full">
      <ResponsiveContainer width="100%" height="100%">
        <BarChart data={chartData} margin={{ top: 20, right: 20, left: -10, bottom: 0 }}>
            <XAxis dataKey="date" tickLine={false} axisLine={false} tickMargin={8} />
            <YAxis tickLine={false} axisLine={false} tickMargin={8} tickFormatter={(value) => `R$${value/1000}k`} />
            <Tooltip 
                cursor={{ fill: 'hsl(var(--accent) / 0.1)' }} 
                content={<ChartTooltipContent 
                    formatter={(value) => `R$ ${value.toLocaleString('pt-BR')}`} 
                    indicator="dot"
                />} 
            />
            <Bar dataKey="sales" fill="var(--color-sales)" radius={[4, 4, 0, 0]} />
        </BarChart>
      </ResponsiveContainer>
    </ChartContainer>
  );
}
