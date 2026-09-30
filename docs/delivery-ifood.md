# Delivery — base interna e contrato pendente

## Implementado nesta rodada

Migration SQLite 10 cria `delivery_inbox`. O módulo `src-tauri/src/db/delivery.rs`
grava lotes atomicamente, preserva o JSON, deduplica por empresa/unidade/provedor/loja/evento
e rejeita conteúdo divergente para um identificador já armazenado. Consulta paginada
por sequência de chegada usa índice de escopo. Limites internos: 500 eventos e 4 MiB
serializados por lote. Nenhuma venda, estoque ou confirmação externa é gerada.

Este módulo é uma base interna, ainda sem comandos Tauri, rotas HTTP ou worker.
Não representa uma integração iFood/99Food pronta. A sequência de chegada não é
o estado do pedido: eventos podem chegar fora de ordem. Não há limpeza automática
da fila nesta etapa; retenção e política de dados pessoais continuam pendentes.

## Próximas etapas internas

1. Configuração autenticada da associação loja externa → empresa/unidade. Nunca
   confiar em tenant/unit enviados por um webhook para estabelecer autorização.
2. Inbox equivalente no backend e transporte autenticado backend → desktop;
   definir um único responsável por pedido, evitando processamento em dois caixas.
3. Worker com processamento transacional, tentativas/backoff e checkpoint persistido.
   Persistir a intenção de acknowledgment para repetir após timeout sem perder eventos.
4. Adaptador da API de eventos, OAuth, deadlines, rate limits e assinatura de webhook.
   Credenciais ficam no backend; o módulo local não autentica requisições externas.
5. Buscar detalhes/virtual-bag para mercado, reconciliar versões e manter estados
   confirmados separados de ações solicitadas. HTTP 202 não confirma o novo estado.
6. Tela real de pedidos, mapeamento de itens, picking/substituições, ações, erros e
   impressão. Conversão em venda precisa ser idempotente e conciliar pagamentos.
7. Testes de falha após commit/antes de ACK, concorrência, eventos fora de ordem,
   cancelamento/alteração e reconciliação. Validar contrato e homologação com a loja.

## Fontes e cuidado com o material enviado

O guia fornecido pelo usuário contém exemplos de Order que diferem do módulo
específico Events. Não misturar seus endpoints e formatos de acknowledgment.
O conector ainda não faz chamadas e não assume que esses contratos são intercambiáveis.

- [Events polling: endpoint e corpo do ACK](https://developer.ifood.com.br/es-CO/docs/food/guides/modules/events/polling-overview)
- [Webhook: duplicidade e ausência de ordem garantida](https://developer.ifood.com.br/en-US/docs/food/guides/modules/events/webhook-overview)
- [Fluxo de pedidos de mercado e virtual-bag](https://developer.ifood.com.br/en-US/docs/groceries/guides/modules/order/workflow)

Para 99Food, aguardar documentação oficial e acesso ao programa de integração;
o identificador de provedor usado nos testes não constitui um conector.
