// src/ai/flows/smart-restock.ts
'use server';

/**
 * @fileOverview A smart restock suggestion AI agent.
 *
 * - smartRestock - A function that suggests optimal restock quantities based on recent sales data.
 * - SmartRestockInput - The input type for the smartRestock function.
 * - SmartRestockOutput - The return type for the smartRestock function.
 */

import {ai} from '@/ai/genkit';
import {z} from 'genkit';

const SmartRestockInputSchema = z.object({
  productId: z.string().describe('The ID of the product to restock.'),
  productName: z.string().describe('The name of the product.'),
  recentSalesData: z
    .string()
    .describe(
      'A string containing recent sales data for the product, including dates and quantities sold.'
    ),
  currentStock: z.number().describe('The current stock level of the product.'),
  minStock: z.number().describe('The minimum stock level of the product.'),
});
export type SmartRestockInput = z.infer<typeof SmartRestockInputSchema>;

const SmartRestockOutputSchema = z.object({
  restockQuantity: z
    .number()
    .describe(
      'The suggested restock quantity for the product, based on the sales data and current stock levels.'
    ),
  reasoning: z
    .string()
    .describe(
      'The reasoning behind the suggested restock quantity, explaining how the sales data and stock levels were considered.'
    ),
});
export type SmartRestockOutput = z.infer<typeof SmartRestockOutputSchema>;

export async function smartRestock(input: SmartRestockInput): Promise<SmartRestockOutput> {
  return smartRestockFlow(input);
}

const prompt = ai.definePrompt({
  name: 'smartRestockPrompt',
  input: {schema: SmartRestockInputSchema},
  output: {schema: SmartRestockOutputSchema},
  prompt: `You are an inventory management expert. Analyze the recent sales data for the product and suggest an optimal restock quantity.

Product Name: {{{productName}}}
Recent Sales Data: {{{recentSalesData}}}
Current Stock: {{{currentStock}}}
Minimum Stock: {{{minStock}}}

Consider the sales trends, current stock, and minimum stock level to determine the restock quantity. Provide a clear reasoning for your suggestion.

Output your reasoning, and the restock quantity.
`,
});

const smartRestockFlow = ai.defineFlow(
  {
    name: 'smartRestockFlow',
    inputSchema: SmartRestockInputSchema,
    outputSchema: SmartRestockOutputSchema,
  },
  async input => {
    const {output} = await prompt(input);
    return output!;
  }
);
