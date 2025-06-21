export type Product = {
  id: string;
  name: string;
  image: string;
  hint: string;
  barcode?: string;
  category: string;
  price: number;
  stock: number;
  minStock: number;
  supplierId?: string;
  expiryDate?: string;
};

export type Employee = {
  id: string;
  name: string;
  role: 'Caixa' | 'Gerente' | 'Estoquista';
  status: 'Ativo' | 'Inativo';
  avatar: string;
  hint: string;
};

export type Expense = {
  id: string;
  description: string;
  category: 'Folha de Pagamento' | 'Contas Fixas' | 'Aluguel' | 'Fornecedores' | 'Serviços' | 'Outros';
  amount: number;
  dueDate: string; // Using string for mock data simplicity: 'dd/MM/yyyy'
  status: 'Pago' | 'Pendente';
};

export type ShiftSummary = {
  employees: Pick<Employee, 'id' | 'name'>[];
  sales: {
    cash: number;
    card: number;
    pix: number;
    total: number;
  };
  cashDrawer: {
    initial: number;
  };
};

export type Category = {
  id: string;
  name: string;
};

export type Supplier = {
  id: string;
  name: string;
  contactInfo?: string;
};
