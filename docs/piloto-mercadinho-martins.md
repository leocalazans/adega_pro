# Piloto Mercadinho Martins — roteiro de campo

## Antes de sair

- Provisionar a loja pelo painel em `/superadmin` ou executar `scripts\provision-martins.cmd` em um CMD. O script pede a senha do SuperAdmin de forma oculta, cria a loja e unidade inicial e mostra um código de ativação de uso único apenas uma vez.
- Levar um notebook com acesso ao painel SuperAdmin, o instalador `CommerceCTRL PDV 0.1.2` e acesso à internet.
- Criar um código de ativação de uso único por terminal no painel; registrar a unidade e o nome físico de cada caixa antes de entregar o código.
- Confirmar que o backend responde `GET /health` e que o terminal usa a URL oficial do backend.
- Não usar o usuário de demonstração como credencial definitiva. No primeiro login local, trocar a senha temporária.

## Em cada unidade

1. Instalar o mesmo executável Windows x64 e ativar com o código daquela unidade.
2. Confirmar o nome do mercado no cabeçalho, o estado Online/Offline e que a licença está ativa.
3. Cadastrar ou sincronizar dois produtos reais de teste, um com EAN impresso e outro por pesquisa.
4. Abrir caixa, registrar uma venda de valor baixo e imprimir somente o **COMPROVANTE DE VENDA — NÃO É DOCUMENTO FISCAL**.
5. Desconectar a rede, registrar outra venda, reconectar e confirmar que a outbox foi esvaziada.
6. Fechar o caixa, reiniciar o PDV e confirmar que a venda, o estoque e o fechamento continuam visíveis. Esse é o teste de recuperação local após backup.

## Scanner de código de barras

- Testar cinco leituras reais: EAN-13, produto com embalagem refletiva e leitura repetida.
- Resultado esperado: o cursor do PDV recebe o código e inclui o item uma vez por bip; nenhuma digitação manual é necessária.
- Registrar modelo/conexão do scanner, quantidade de leituras corretas e qualquer duplicação/perda.

## Impressora

- Registrar marca, modelo, conexão (USB/rede), driver e nome exibido pelo Windows.
- Testar impressão gráfica do comprovante e, se térmica, ESC/POS/rede com corte/alinhamento.
- Resultado esperado: logo opcional, itens, total, pagamento e o aviso não fiscal legíveis.
- Se falhar: manter a venda gravada, capturar o nome da fila/porta e testar a fila do Windows; não repetir a venda só para imprimir.

## Balança Prix Wi-Fi/rede

1. Registrar modelo, IP, máscara, gateway, porta e se a comunicação é por protocolo TCP/IP, arquivo de etiquetas ou integração do fabricante.
2. Confirmar conectividade da rede local e consultar no manual da balança o formato do EAN/PLU embutido no código de barras.
3. Imprimir e ler três etiquetas de peso: produto normal, fração e valor alto.
4. Resultado esperado nesta etapa: o PDV lê o EAN da etiqueta; a extração automática de peso só deve ser habilitada após validar o prefixo/formato específico da Prix usada.

Não configurar uma regra de peso por suposição: uma regra errada altera quantidade e preço no caixa.

## Feedback a devolver

Para cada unidade/terminal, registrar: horário, rede disponível, ativação, login, scanner (leituras corretas/total), impressora (modo e resultado), balança (modelo/formato de etiqueta), venda offline/online, pendências e foto do comprovante sem dados pessoais.
