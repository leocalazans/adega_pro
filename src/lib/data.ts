import type { Product, Employee, Expense, ShiftSummary, Promotion } from './types';

// Using a Map for easier lookups
const productsData: Product[] = [
    { id: 'PROD001', name: "Vinho Tinto Suave", price: 30.00, stock: 8, minStock: 10, category: 'Vinhos', image: "https://placehold.co/150x150.png", hint: "wine bottle", expiryDate: '15/12/2025', barcode: "78900001", recentSales: "Vendas na última semana: 15 unidades.", revenue: 1470.00 },
    { id: 'PROD002', name: "Cerveja Artesanal IPA", price: 15.00, stock: 40, minStock: 25, category: 'Cervejas', image: "https://placehold.co/150x150.png", hint: "beer bottle", expiryDate: '20/08/2024', barcode: "78900002", recentSales: "Vendas na última semana: 18 unidades.", revenue: 4160.00 },
    { id: 'PROD003', name: "Whisky 12 Anos", price: 120.00, stock: 15, minStock: 5, category: 'Destilados', image: "https://placehold.co/150x150.png", hint: "whiskey bottle", barcode: "78900003", revenue: 5400.00 },
    { id: 'PROD004', name: "Gin Importado", price: 130.00, stock: 12, minStock: 10, category: 'Destilados', image: "https://placehold.co/150x150.png", hint: "gin bottle", barcode: "78900004", revenue: 3600.00 },
    { id: 'PROD005', name: "Água Tônica", price: 5.00, stock: 3, minStock: 20, category: 'Não Alcoólicos', image: "https://placehold.co/150x150.png", hint: "soda can", expiryDate: '30/08/2024', barcode: "78900005", recentSales: "Vendas na última semana: 45 unidades.", revenue: 1050.00 },
    { id: 'PROD006', name: "Energético", price: 8.00, stock: 50, minStock: 20, category: 'Não Alcoólicos', image: "https://placehold.co/150x150.png", hint: "energy drink", expiryDate: '30/07/2024', barcode: "78900006" },
    { id: 'PROD007', name: "Saca-rolhas", price: 25.00, stock: 2, minStock: 5, category: 'Acessórios', image: "https://placehold.co/150x150.png", hint: "corkscrew", barcode: "78900007", recentSales: "Vendas na última semana: 1 unidade." },
    { id: 'PROD008', name: "Cerveja Pilsen Pack 6", price: 22.00, stock: 11, minStock: 15, category: 'Cervejas', image: "https://placehold.co/150x150.png", hint: "beer pack", barcode: "78900008", recentSales: "Vendas na última semana: 22 unidades." },
    { id: 'PROD009', name: "Salgadinho de Queijo", price: 7.50, stock: 25, minStock: 15, category: 'Snacks', image: "https://placehold.co/150x150.png", hint: "snack bag", expiryDate: '01/07/2024' },
    { id: 'PROD010', name: "Maço de Cigarros", price: 12.00, stock: 100, minStock: 30, category: 'Tabacaria', image: "https://placehold.co/150x150.png", hint: "cigarette pack" },
    { id: 'PROD011', name: "Isqueiro", price: 4.00, stock: 30, minStock: 10, category: 'Tabacaria', image: "https://placehold.co/150x150.png", hint: "lighter" },
];

const employeesData: Employee[] = [
  { id: 'EMP01', name: 'Ana Silva', role: 'Caixa', status: 'Ativo', avatar: 'https://placehold.co/40x40.png', hint: 'woman avatar' },
  { id: 'EMP02', name: 'Bruno Costa', role: 'Gerente', status: 'Ativo', avatar: 'https://placehold.co/40x40.png', hint: 'man avatar' },
  { id: 'EMP03', name: 'Carlos Dias', role: 'Caixa', status: 'Inativo', avatar: 'https://placehold.co/40x40.png', hint: 'man portrait' },
  { id: 'EMP04', name: 'Daniela Souza', role: 'Estoquista', status: 'Ativo', avatar: 'https://placehold.co/40x40.png', hint: 'woman portrait' },
  { id: 'EMP05', name: 'Eduardo Lima', role: 'Caixa', status: 'Ativo', avatar: 'https://placehold.co/40x40.png', hint: 'male avatar' },
];

const expensesData: Expense[] = [
  { id: 'EXP001', description: 'Pagamento de Salário - Ana Silva', category: 'Folha de Pagamento', amount: 1800.00, dueDate: '05/07/2024', status: 'Pago' },
  { id: 'EXP002', description: 'Conta de Energia Elétrica', category: 'Contas Fixas', amount: 450.75, dueDate: '10/07/2024', status: 'Pendente' },
  { id: 'EXP003', description: 'Aluguel do Ponto Comercial', category: 'Aluguel', amount: 3500.00, dueDate: '08/07/2024', status: 'Pendente' },
  { id: 'EXP004', description: 'Compra de Estoque - Bebidas ABC', category: 'Fornecedores', amount: 1200.00, dueDate: '28/06/2024', status: 'Pago' },
  { id: 'EXP005', description: 'Conta de Água', category: 'Contas Fixas', amount: 150.20, dueDate: '12/07/2024', status: 'Pendente' },
  { id: 'EXP006', description: 'Serviço de Contabilidade', category: 'Serviços', amount: 600.00, dueDate: '15/07/2024', status: 'Pendente' },
];

const shiftSummaryData: ShiftSummary = {
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

const promotionsData: Promotion[] = [
  { id: 'PROMO01', productName: "Combo Whisky + Energético", price: "R$ 130,00", image: "https://placehold.co/800x600.png", hint: "whiskey drinks", highlight: "OFERTA IMPERDÍVEL" },
  { id: 'PROMO02', productName: "Cerveja Pilsen Pack 6", price: "R$ 19,99", image: "https://placehold.co/800x600.png", hint: "beer party" },
  { id: 'PROMO03', productName: "Vinho Tinto Suave", price: "Pague 2 Leve 3", image: "https://placehold.co/800x600.png", hint: "wine glasses", highlight: "SÓ HOJE" },
  { id: 'PROMO04', productName: "Gin com Tônica", price: "Dose Dupla", image: "https://placehold.co/800x600.png", hint: "cocktail gin" },
];


// Simulate an API call with a short delay
const apiDelay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

// --- Service Functions ---

export async function getProducts(): Promise<Product[]> {
  await apiDelay(50);
  return productsData;
}

export async function getProductById(id: string): Promise<Product | undefined> {
  await apiDelay(20);
  return productsData.find(p => p.id === id);
}

export async function getEmployees(): Promise<Employee[]> {
  await apiDelay(50);
  return employeesData;
}

export async function getExpenses(): Promise<Expense[]> {
  await apiDelay(50);
  return expensesData;
}

export async function getShiftSummary(): Promise<ShiftSummary> {
    await apiDelay(50);
    return shiftSummaryData;
}

export async function getPromotions(): Promise<Promotion[]> {
    await apiDelay(20);
    return promotionsData;
}
