# Testes e aceite do CommerceCTRL

Comando único: `powershell -ExecutionPolicy Bypass -File scripts/test-all.ps1`.

## Portões automatizados ativos

- Svelte: testes unitários do formatador, rotas e adaptador de integração; verificação TypeScript/Svelte; build de produção.
- SQLite/financeiro/pagamentos: 8 testes unitários e 1 teste de integração cobrindo migrações e seeds, ciclo de caixa, sangria, suprimento, fechamento cego, venda atômica, rollback, baixa de estoque, cache, outbox, cadastros operacionais, fidelidade, dashboard, totais por pagamento e geração/validação PIX.
- PostgreSQL/backend: 17 verificações compatíveis com Windows PowerShell 5+ cobrindo saúde, autenticação obrigatória, seeds, materialização de produto, registros operacionais, atualização parcial segura, promoções, venda com baixa de estoque, idempotência e isolamento real entre duas unidades; os dados temporários são removidos ao final.
- Containers: build e health checks do backend e PostgreSQL.
- Modo TV: promoções ativas vêm do SQLite, são atualizadas automaticamente a cada 5 segundos e alternadas a cada 10 segundos, preservando a promoção atual quando possível.
- PDV: o carrinho bloqueia quantidade acima do estoque local, produtos zerados ficam indisponíveis e o saldo visível é atualizado imediatamente após a venda, além da validação transacional no SQLite.
- Sincronização: catálogo, estoque e estoque mínimo retornados pelo PostgreSQL são aplicados ao SQLite; o indicador online só é ativado após uma chamada de rede bem-sucedida.

## Pendências que impedem declarar paridade total

- Exclusão lógica de produtos e validação visual de conflitos; produtos, clientes, funcionários, fornecedores, despesas e promoções já podem ser editados e sincronizados, com controles de status operacionais.
- Relatórios consolidados na nuvem (o relatório local por período, terminal, pagamento e produto, com CSV e impressão, já está coberto).
- Autenticação de usuários, funções e permissões por unidade.
- Confirmação automática de PIX/cartão com PSP e webhook assinado; o PDV já gera BR Code PIX local, exige confirmação explícita do crédito e, para cartão, autorização e confirmação do operador.
- Fiscal em homologação real e certificado A1.
- Testes de UI/E2E no executável Windows, dependentes da instalação do Windows SDK com `dbghelp.lib`.

Esses itens devem permanecer explícitos: nenhum portão pode ser marcado como aprovado sem teste reproduzível.
# Suíte automatizada

Execute todos os portões com:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\test-all.ps1
```

A suíte inclui testes unitários do frontend com cobertura HTML em
`frontend-svelte/coverage`, testes de banco SQLite e regras financeiras, testes do
desktop Rust, testes do backend, integração PostgreSQL/Docker, verificação Svelte e
build do frontend. Código de interface e integrações com hardware precisam também dos
testes de integração/E2E descritos abaixo; cobertura unitária não substitui periféricos.
