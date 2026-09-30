# Catálogo comercial e homologação

## Oferta publicada

| Plano | Mensalidade por loja | Entrega comercial |
| --- | ---: | --- |
| Começar | R$ 100 | PDV, estoque, relatórios básicos e comprovante não fiscal. |
| Fiscal | R$ 180 | Base do plano Começar e recursos fiscais somente após a homologação aplicável. |
| PDV Premium | R$ 250 | Multiunidade, TV de ofertas, controles ampliados e prioridade em integrações liberadas. |

O preço do software nunca deve esconder equipamento. A proposta separa mensalidade do plano, implantação, compra e aluguel mensal de comodato.

## Modalidades de equipamento

1. **Venda:** cada componente possui SKU, fornecedor, custo, margem, preço e compatibilidade cadastrados pelo SuperAdmin antes de ser ofertado.
2. **Comodato:** o mesmo componente possui valor mensal de aluguel, prazo mínimo, reposição e condição de devolução. O aluguel aparece em linha própria na proposta e na cobrança.

Não cadastrar preço de Mercado Livre por cópia manual. O catálogo deve guardar URL/origem e data de consulta, e o administrador confirma preço, disponibilidade e garantia antes da proposta.

## Pacote a verificar por loja

- computador ou mini PC Windows x64;
- leitor de código de barras USB/HID;
- impressora térmica compatível com Windows/ESC-POS;
- gaveta de dinheiro compatível com a impressora;
- rede estável e nobreak quando necessário;
- balança Prix somente com modelo, IP, porta, protocolo e formato de retorno confirmados no local.

O código atual ainda não lê balança Prix automaticamente. A ativação dessa função é bloqueada até o teste de rede e protocolo no estabelecimento.

## Integrações em mapa de homologação

| Integração | Estado comercial | Condição para ativar |
| --- | --- | --- |
| iFood | Mapeada, não disponível para produção | Contrato/credencial do parceiro, escopo de API e homologação por loja. |
| 99Food | Mapeada, não disponível para produção | Confirmação de API, credencial e homologação. |
| Zé Delivery | Mapeada, não disponível para produção | Elegibilidade do parceiro, credencial e homologação. |
| Emissão fiscal | Em homologação | UF, certificado/credencial, regras fiscais e testes aprovados. |
| Aplicativo Android | Planejado | APK assinado, testes e política de distribuição concluídos. |

## Governança no SuperAdmin

Antes de liberar a venda, o SuperAdmin deve controlar por tenant: plano, modalidade (venda ou comodato), componentes contratados, valores aprovados, data de vigência e flags de recurso. Flags elegíveis incluem `fiscal`, `integracoes_delivery`, `tv_ofertas`, `multiunidade`, `android_companion` e `balanca_prix`; todas começam desligadas até a respectiva homologação.
