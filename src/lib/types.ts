export type Product = {
  id: string;
  name: string;
  photoUrl: string;
  barcode?: string;
  category: Category;
  costPrice: number;
  salePrice: number;
  stock: number;
  minStock: number;
  supplier?: Supplier;
  expiryDate?: string;
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
