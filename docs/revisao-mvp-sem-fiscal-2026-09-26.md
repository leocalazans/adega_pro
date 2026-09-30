# Revisão do MVP sem emissão fiscal — 26/09/2026

## Escopo considerado

Este corte opera como PDV **não fiscal**. O comprovante continua identificado como
`COMPROVANTE DE VENDA — NÃO É DOCUMENTO FISCAL`. NFC-e, SAT, certificado e SEFAZ
não fazem parte deste MVP.

## Pronto no código

- catálogo, busca por EAN e venda offline no SQLite;
- abertura, sangria, suprimento e fechamento de caixa;
- confirmação manual de cartão e PIX (com suporte Asaas quando configurado);
- comprovante não fiscal com nome e logo configurável da loja;
- impressão ESC/POS por TCP, porta local ou fila RAW do Windows, com retentativa;
- produtos, estoque, clientes, funcionários, fornecedores, despesas e promoções;
- relatórios de vendas e visão consolidada das quatro unidades no backend;
- fila offline-first para venda, estoque, cadastros, caixa e ponto;
- relógio de ponto local com sincronização posterior;
- etiquetas CODE128 com quantidade e preço, usando o diálogo de impressão do Windows;
- modo TV local com rotação, rádio, identidade do cliente e imagem de produto demo;
- identidade do cliente persistida na nuvem e cache WebP local;
- bloqueio de retirada do carrinho e fechamento do quiosque por senha Argon2;
- endpoint e cliente de atualização assinada preparados.

## Validações executadas sem gerar build

- `svelte-check`: 0 erros (avisos visuais preexistentes no PDV);
- `vitest`: 3 arquivos e 7 testes aprovados;
- `cargo fmt --check`: aprovado no desktop e no backend.

## Implementado nesta revisão

- login por funcionário, sessão local, perfis e permissões, provisionamento pelo
  administrador e troca obrigatória da senha temporária no primeiro acesso;
- workflow de release Windows assinado, publicação do artefato e `latest.json` no
  Supabase Storage e acionamento opcional do deploy hook do Render;
- impressão ESC/POS/RAW e alternativa gráfica pelo diálogo do Windows;
- imagem WebP individual por produto, enviada pela fila offline e persistida no
  PostgreSQL/Supabase, com cache no terminal para continuar funcionando offline;
- criação e restauração validada do SQLite pela tela de configurações;
- financeiro operacional, caixa, despesas, relatórios e demonstrativo gerencial;
- licença assinada com renovação online, tolerância offline e desbloqueio assinado.

## Dependências externas para liberar produção

1. Preencher os segredos e endereços definitivos de Render/Supabase no ambiente e no
   GitHub Actions; sem essas credenciais nenhum repositório pode publicar em nome do cliente.
2. Executar o roteiro `scripts/hardware-acceptance.ps1` com impressora e leitor físicos.
   Nesta estação só foram encontradas filas virtuais do Windows.
3. Contabilidade societária/fiscal completa exige escopo do contador (plano de contas,
   regime tributário, conciliação, lançamentos dobrados, DRE e balanço). O módulo atual
   é financeiro gerencial e não deve ser anunciado como escrituração contábil.

## Comandos para o responsável gerar e instalar

```powershell
cd D:\global\CommerceCTRL\frontend-svelte
npm install
npm run check
npm test

cd D:\global\CommerceCTRL\src-tauri
cargo tauri build
```

Antes do build de produção, substitua os endpoints locais do updater/backend pelos
endereços do Render e defina as credenciais de terminal/tenant por ambiente. Não
publique a chave privada do updater nem credenciais do Asaas no repositório.
