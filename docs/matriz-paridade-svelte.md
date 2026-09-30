# Matriz de paridade — React/Next para Svelte 5

Esta matriz é o contrato de aceite da migração. A implementação Svelte só substitui a tela atual quando todos os itens da rota estiverem cobertos por teste.

## Rotas

| Rota | Perfil | Funções atuais a preservar | Integração real exigida |
|---|---|---|---|
| `/` | Admin | indicadores, gráfico, ranking e estoque baixo | vendas e estoque SQLite |
| `/pos` | Admin/Caixa | busca, carrinho, quantidade, remoção, meios de pagamento, troco, câmera, tela cheia e atalhos F2/F4/F8/F11/Esc/Enter | `search_product`, `search_by_ean`, `record_sale`, `generate_pix`, `print_receipt`, eventos de código de barras |
| `/products` | Admin | catálogo, situação de estoque, validade, criar, editar, duplicar e excluir | `list_products` e novos comandos CRUD/importação CSV |
| `/restock` | Admin | prioridade por estoque, progresso e sugestão IA | vendas recentes, estoque mínimo e sugestão não vinculante |
| `/reports` | Admin | relatórios financeiros | período, terminal, pagamento, produto, estoque baixo, exportação e impressão |
| `/expenses` | Admin | totais, pendências, cadastro, edição, pagamento e exclusão | novo CRUD SQLite/outbox |
| `/suppliers` | Admin | cadastro e gestão de fornecedores | novo CRUD SQLite/outbox |
| `/employees` | Admin | cadastro, cargos, atividade e ativar/desativar | autenticação e permissões reais |
| `/contacts` | Admin | cadastro e gestão de clientes | busca, cadastro, pontos e resgate |
| `/cash-closing` | Admin/Caixa | funcionário, resumo por pagamento, contagem e diferença | `cash_status`, `cash_open`, `cash_sangria`, `cash_suprimento`, `cash_fechamento_cego`, `cash_close` |
| `/display` | Admin | promoções, vídeo, áudio, mute e alternância de modo | sessão do terminal e atualização do carrinho em tempo real |

## Contrato Tauri que não pode quebrar

- Produtos: `search_product`, `search_by_ean`, `list_products`, `adjust_stock`.
- Venda: `record_sale` com UUID, terminal, baixa de estoque e outbox na mesma transação.
- Sincronização: `pending_outbox`, `sync_now`, `sync_status`.
- Pagamentos: `generate_pix`; pagamento só muda para confirmado após resposta válida do PSP.
- Caixa: `cash_status`, `cash_open`, `cash_close`, `cash_sangria`, `cash_suprimento`, `cash_fechamento_cego`.
- Impressão: `print_receipt`, `print_text`, `print_status`, `print_retry`.
- Leitor: `emit_barcode` e evento `barcode-scanned`.
- Fidelidade: `search_customers`, `list_customers`, `upsert_customer`, `add_customer_points`, `redeem_customer_points`.
- Fiscal: `emit_nfe` continua fora do caminho crítico do MVP até homologação.

## Portões de aceite

1. Banco temporário: abrir caixa, vender, baixar estoque, gerar outbox, fechar e reabrir sem perda.
2. Venda recusada não altera estoque, caixa ou outbox.
3. Repetir UUID remoto não duplica venda nem movimento de estoque.
4. Sem rede, toda função local continua operando; reconexão drena a outbox.
5. Leitor e atalhos funcionam sem mouse e sem atraso perceptível.
6. PIX manual não aparece como confirmado automaticamente; PIX dinâmico depende de webhook assinado.
7. Cada rota passa por comparação visual e funcional antes da remoção da equivalente React.
