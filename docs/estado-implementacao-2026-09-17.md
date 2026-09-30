# Estado técnico — 17/09/2026

Este registro substitui afirmações anteriores de conclusão que não foram acompanhadas de validação ponta a ponta.

## Implementado, com validação em andamento

- Intenções PIX Asaas no PostgreSQL e endpoints de criação/consulta; webhook autenticado por token `asaas-access-token` (não é assinatura criptográfica do corpo).
- Licenças Ed25519: emissão administrativa, consulta de renovação, expiração/carência e importação de JSON no desktop. O pendrive transporta um arquivo copiável; não é um token físico anticópia.
- CPF no PDV e fila fiscal gravada na transação de venda. A fila permanece `pending_configuration`; isso não equivale a uma NFC-e autorizada nem a contingência fiscal válida.
- Código de assinatura fiscal fictícia removido; comando de emissão retorna indisponibilidade explícita.

## Trabalho interno ainda necessário

- Autenticação de operadores e autorização por função/unidade; o perfil fixo de administrador da interface ainda não representa uma sessão autenticada.
- Pagamentos: escopo das credenciais por empresa/unidade, recuperação de timeouts/idempotência, validação de valor e estado dos webhooks, associação única entre pagamento confirmado e venda, estorno e testes completos de concorrência. Ainda não liberar para produção.
- Licenças: suspensão administrativa, renovação periódica automática, rejeição de replay, recuperação controlada de relógio, política comercial definitiva e chaves de produção. Os valores de desenvolvimento publicados no Compose não servem para produção. Proteção local contra administrador do Windows não é inviolável.
- Fiscal: cadastro tributário, assinatura XMLDSig real, validação de schema, numeração, DANFE, autorização/cancelamento e transmissão de contingência. A ausência de credenciais não impede desenvolver e testar estes componentes internamente.
- Delivery: foram criadas tabelas; conectores iFood/99Food, telas e fluxos operacionais ainda não foram concluídos.
- Instalador, testes E2E Windows, recuperação de backup e revisão de segurança completa.

## Dependências externas

- Certificado A1, CSC, credenciamento e homologação SEFAZ-SP.
- Configuração de conta Asaas do mercadinho e endpoint HTTPS para callbacks reais.
- Credenciais/homologação iFood e acesso oficial 99Food; TEF adiado pelo usuário.

Nada desta lista deve ser declarado aprovado somente porque `cargo check` ou o build passou.

## Verificação desta rodada

- Nova imagem backend compilada e iniciada; backend e PostgreSQL saudáveis.
- Script HTTP/PostgreSQL: 24 verificações aprovadas, incluindo emissão/renovação de licença e rejeição de webhook sem token.
- Testes Rust executados nativamente no Windows com SDK: 11 unitários e 1 integração aprovados, incluindo adulteração da licença, expiração, arquivo UTF-8 com BOM, instalação diferente e relógio retrocedido.
- Svelte: 7 testes aprovados e análise estática sem erros/avisos.
- Correções do PDV: atalhos numéricos não alteram pagamento durante digitação do CPF; clique concorrente ignora uma confirmação já em processamento; QR Asaas não exige chave PIX local; erro de criação não gera automaticamente outra cobrança local.

## Rodada seguinte — sincronização validada

Corrigida a materialização de vendas para impedir segunda baixa de estoque quando o mesmo UUID de venda chega em outro evento de sincronização. O reenvio compara unidade, terminal, total, pagamento, data e itens; divergência retorna HTTP 409. Linhas repetidas do mesmo EAN agora somam as quantidades no movimento de estoque.

O script de integração foi ampliado de 24 para 26 verificações e passou a exercitar itens repetidos, reenvio com novo evento e conflito de conteúdo. Em 17/09/2026, todas as 26 verificações passaram contra a nova imagem. Build Docker e `cargo check` local concluídos com sucesso; backend e PostgreSQL saudáveis após a execução. Estes resultados cobrem a sincronização testada e não encerram as pendências internas listadas acima.

## Proteções adicionais do SQLite — validadas

A venda agora recusa produtos arquivados dentro da própria transação, mesmo se ainda estiverem em um carrinho antigo. Quantidades não finitas e valores que excedem o limite numérico são recusados antes de gravar. Testes verificam que uma falha em um item desfaz a baixa de outros itens e não cria venda, outbox ou registro fiscal.

Suíte local executada no Windows: 13 testes unitários e 1 integração financeira aprovados. O executável/instalador ainda precisa ser recompilado para distribuir estas alterações.

## Atualização de schema SQLite — validada

Cada migration agora executa alterações de estrutura/dados e atualização de `user_version` na mesma transação. Uma falha no meio da migration desfaz as alterações daquela versão e permite tentar novamente. Teste com falha SQL induzida comprovou rollback, preservação dos dados, versão anterior e reexecução idempotente após correção. Suíte Windows atual: 14 testes unitários e 1 integração financeira aprovados; containers permanecem saudáveis.

## CPF — validação local concluída

Identificado que o adaptador HTTP descartava o CPF da venda. O campo agora segue pelo adaptador e pelo servidor local até a fila fiscal. Validação e normalização foram centralizadas no SQLite para cobrir ambos os caminhos (Tauri e HTTP): CPF inválido ou com letras é recusado antes da baixa de estoque; CPF válido é persistido sem pontuação. A verificação duplicada em `main.rs` foi removida.

15 testes unitários Rust e 1 integração passaram. O teste do adaptador frontend verifica o envio do CPF: 7 testes da interface passaram; Svelte check terminou sem erros ou avisos. `cargo check` Tauri terminou com sucesso e 36 avisos. Esta é uma checagem de compilação, não um novo instalador nem um teste E2E do executável Windows.

## Delivery — inbox local validada

Adicionada migration 10 e módulo interno de recebimento persistente de eventos. Isolamento por empresa, unidade, provedor e loja externa; deduplicação com rejeição de conteúdo divergente; transação por lote e paginação por índice. Não altera estoque/vendas, não confirma recebimento ao provedor e ainda não está ligado a conector, worker ou tela.

Suíte completa após a alteração: 19 testes unitários e 1 integração aprovados. Os quatro testes novos cobrem duplicidade/conflito com rollback, isolamento/paginação, limites de entrada e persistência após reabrir o arquivo SQLite. Um identificador inicialmente incorreto da migration foi corrigido antes desta execução aprovada. Contratos externos e próximos passos registrados em `docs/delivery-ifood.md`.

`cargo check` Tauri aprovado nesta rodada com 43 avisos, incluindo código da inbox ainda sem consumidor. Formatação Rust aprovada. Backend e PostgreSQL continuam em execução com healthcheck saudável. Não foi gerado novo instalador.

## Multiunidade — painel do proprietário em implementação

O backend passou a ter sobrescritas de preço, estoque mínimo e disponibilidade por unidade, sem alterar o cadastro mestre do produto. Promoções podem ser globais (todas as unidades) ou vinculadas a unidades selecionadas. O endpoint do painel consolidado é protegido por credencial de proprietário separada da credencial de terminal; ele devolve faturamento, vendas, itens, SKUs com estoque crítico por unidade e ranking de produtos por faturamento.

A interface de relatórios passa a consumir esse painel no desktop autenticado. Ainda é necessário cadastrar as quatro unidades reais, seus terminais e as chaves de produção, e executar a homologação da nova migration/endpoints antes de liberar esse acesso ao cliente.

## Modo TV por unidade — em validação

O desktop passa a iniciar uma porta de TV separada em `0.0.0.0:9002`, sem rotas de venda, estoque ou administração. Cada instalação gera e guarda um token de leitura para a sua URL de TV; logo, um caixa de uma unidade só entrega as promoções do seu SQLite local. A tela consulta a cada cinco segundos e mantém a última oferta caso a conexão oscile.

O backend recebeu canais online vinculados a empresa/unidade e protegidos por token. A URL pública retorna exclusivamente promoções globais ou habilitadas para aquela unidade. Não há publicação externa ainda: para usar fora da rede da loja é necessário domínio HTTPS, proxy reverso e configuração de `PUBLIC_BASE_URL`; não expor a porta Docker diretamente na internet.
