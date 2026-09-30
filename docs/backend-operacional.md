# Backend operacional do CommerceCTRL

O CommerceCTRL terá backend próprio. A implementação recomendada para o MVP é um serviço Rust com Axum, PostgreSQL e Redis, separado do aplicativo Tauri e sem qualquer dependência do AutoControl.

## Contrato mínimo

- Autenticação: usuário, empresa (`tenant`) e terminal.
- Todo evento enviado pelo caixa inclui `tenant_id`, `terminal_id` e UUID idempotente.
- `POST /api/v1/sync/outbox` recebe eventos de venda, movimentação de estoque, cliente e caixa. Repetir o mesmo UUID deve retornar sucesso sem duplicar dados.
- `GET /api/v1/sync/products` entrega o catálogo do tenant, com cursor/versionamento.
- O estoque central é uma soma de movimentos imutáveis; nunca é sobrescrito pelo valor de outro caixa.
- Cada terminal mantém SQLite e outbox local. Sem URL configurada, ele permanece offline e não tenta uma API externa.

## PIX automático

O backend criará cobranças dinâmicas no PSP escolhido e receberá webhook assinado. Apenas um webhook validado altera a venda para `paid`; o cliente nunca confirma uma venda sozinho pelo front-end.

Variáveis previstas:

```text
COMMERCECTRL_API_URL=https://api.seu-dominio.com
DATABASE_URL=postgres://...
REDIS_URL=redis://...
PIX_PROVIDER=mercadopago|asaas|pagbank|picpay
```
