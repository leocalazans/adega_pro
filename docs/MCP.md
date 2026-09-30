# CommerceCTRL — contexto MCP para testes e atualizações

Este arquivo é a fonte de contexto para agentes, clientes MCP e outras ferramentas
que venham a testar, corrigir ou atualizar o CommerceCTRL. Antes de modificar o
projeto, leia também:

- `docs/revisao-mvp-sem-fiscal-2026-09-26.md`
- `docs/producao-render-supabase.md`
- `docs/testes-e-aceite.md`
- `docs/blueprint.md`
- `.env.example`

## Escopo atual

O MVP é um PDV não fiscal, offline-first e multiunidade. Emissão fiscal, TEF e
integração direta com adquirentes não fazem parte do aceite atual. O comprovante deve
continuar identificado como `COMPROVANTE DE VENDA — NÃO É DOCUMENTO FISCAL`.

Não substituir integrações reais por dados fictícios. Seeds são permitidos apenas no
tenant de demonstração Mercadinho Martins.

## Componentes

- `frontend-svelte`: interface Svelte e adaptador IPC/API local.
- `src-tauri`: aplicativo Windows, SQLite, impressão, licença e sincronização.
- `backend`: API Rust/Axum e PostgreSQL/Supabase.
- `core-tests`: cenários financeiros e SQLite desacoplados da interface.
- `.github/workflows/release-windows.yml`: release e atualização assinada.
- `scripts/test-all.ps1`: portão automatizado completo.
- `scripts/hardware-acceptance.ps1`: aceite na máquina e periféricos reais.

## Funções que não podem regredir

1. Login individual, primeiro acesso, troca de senha, sessão e permissões por cargo.
2. Abertura de caixa, venda, sangria, suprimento e fechamento cego.
3. Busca por EAN, leitor operando sem foco, catálogo, estoque e exclusão lógica.
4. PIX manual/Asaas, dinheiro e cartão com confirmação do operador.
5. Comprovante não fiscal com logo opcional e impressão ESC/POS, RAW ou gráfica.
6. Operação offline, outbox persistente, reenvio idempotente e conflitos visíveis.
7. Separação por tenant e unidade, inclusive preços, estoques, promoções e modo TV.
8. Imagens WebP de produto e identidade do cliente com disponibilidade offline.
9. Backup, validação e restauração do banco local.
10. Licença assinada, renovação online, tolerância offline, bloqueio e arquivo de
    desbloqueio vinculado à instalação.
11. Atualização Windows assinada, manifesto `latest.json` e publicação no Supabase.
12. Funcionários, ponto, clientes, fidelidade, fornecedores, despesas e relatórios.

## Portão obrigatório antes de aceitar alterações

No PowerShell, a partir da raiz do repositório:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\test-all.ps1
```

Execuções isoladas:

```powershell
npm --prefix frontend-svelte run check
npm --prefix frontend-svelte run test:coverage
npm --prefix frontend-svelte run build

$sdk = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\Lib' -Directory |
  Sort-Object Name -Descending |
  ForEach-Object { Join-Path $_.FullName 'um\x64' } |
  Where-Object { Test-Path (Join-Path $_ 'DbgHelp.Lib') } |
  Select-Object -First 1
if ($sdk) { $env:LIB = "$sdk;$env:LIB" }

cargo test --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path backend/Cargo.toml
cargo test --manifest-path core-tests/Cargo.toml --locked
```

Baseline em 26/09/2026:

- frontend: 21 testes aprovados;
- desktop: 31 testes aprovados;
- backend: 5 testes aprovados;
- total principal: 57 testes, sem falhas;
- `svelte-check`: zero erros;
- cobertura do adaptador frontend: 61,4% statements, 54,54% branches,
  85,18% functions e 56,54% lines.

Uma atualização não pode reduzir os limites configurados em
`frontend-svelte/vite.config.ts`. Código novo com regra de negócio deve chegar com
teste de sucesso, validação, falha esperada e persistência/reabertura quando aplicável.

## Testes que exigem ambiente externo

Não declarar estes itens aprovados sem evidência real:

- impressão em impressora física;
- leitura pelo scanner físico;
- perda e retorno de internet;
- instalação e atualização assinada em outro Windows;
- publicação no Render/Supabase;
- pagamento Asaas sandbox e recebimento do webhook.

Use `scripts/hardware-acceptance.ps1` na máquina do mercado e anexe ao relatório o
modelo dos dispositivos, versão do aplicativo, horário, resultado e erro observado.

## Regras para futuras atualizações

- Preservar o SQLite como fonte operacional local e a outbox offline-first.
- Toda tabela e evento cloud deve conter e validar `tenant_id` e `unit_id`.
- Não enviar chave privada, service role, credencial Asaas ou chave de assinatura ao
  frontend ou ao repositório.
- Alterações de banco exigem migration incremental; nunca apagar banco do cliente.
- Atualização desktop deve ser assinada e validar assinatura antes de instalar.
- Manter cópia recuperável antes de migrations e restaurações.
- Não chamar comprovante comum de nota ou cupom fiscal.
- Não alterar ou remover dados existentes para fazer um teste passar.
- Não usar `git reset --hard`, `git checkout --` ou limpeza destrutiva do workspace.

## Formato do relatório do agente/MCP

Ao concluir uma alteração, registrar:

```text
Objetivo:
Arquivos alterados:
Migrations adicionadas:
Testes adicionados:
Comandos executados:
Resultados e cobertura:
Riscos ou funções não testadas:
Dependências externas pendentes:
Procedimento de reversão:
```

O agente deve distinguir claramente entre teste unitário, integração automatizada e
aceite físico. Ausência de falha automatizada não equivale a homologação do hardware.

## Prompt curto para retomada

```text
Trabalhe no CommerceCTRL em D:\global\CommerceCTRL. Leia docs/MCP.md por completo e
siga seus portões de qualidade. Preserve alterações existentes e o desenho
offline-first/multitenant. Antes de editar, identifique as funções afetadas. Depois,
adicione testes proporcionais, execute scripts/test-all.ps1 e relate resultados,
cobertura, riscos, dependências externas e reversão. Não implemente fiscal ou TEF sem
solicitação explícita.
```
