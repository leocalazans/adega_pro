# Base de privacidade e LGPD — CommerceCTRL

Status: modelo operacional. Revisar com assessoria jurídica antes de produção.

## Papéis

- Cada mercado é, em regra, o **controlador** dos dados de seus consumidores e funcionários.
- CommerceCTRL atua como **operador** para hospedagem, sincronização, suporte e manutenção, conforme contrato.
- Para os próprios dados de cadastro, cobrança e suporte dos mercados, CommerceCTRL é controlador.

## Dados mínimos e finalidade

- Cadastro do mercado: nome, CNPJ, contato e unidades, para contrato, suporte e licenciamento.
- Operação: produtos, vendas, estoque, caixa e documentos fornecidos pelo mercado, para executar o PDV e relatórios.
- Acesso: e-mail/usuário, hash de senha e auditoria de ações administrativas, para segurança e rastreabilidade.
- Não registrar dados de cartão ou segredo de PSP; pagamentos usam provedores contratados.

## Controles técnicos obrigatórios

- TLS em trânsito, RLS no Supabase, segregação por tenant e menor privilégio.
- Argon2 para senhas; tokens/códigos de ativação persistidos somente como hash.
- Secrets apenas no servidor/CI; nunca no instalador, frontend ou repositório.
- Backups cifrados, testes de restauração e logs de auditoria com acesso restrito.

## Retenção e descarte

- Dados fiscais, contábeis e de vendas: conforme prazo legal e orientação do contador do mercado.
- Logs técnicos e auditoria: prazo definido em contrato, com acesso restrito.
- Códigos de ativação expirados/revogados: manter só o hash e evento de auditoria pelo prazo de segurança definido.
- Ao encerrar contrato, exportar dados ao controlador e eliminar/anonomizar depois do prazo legal e contratual.

## Atendimento e incidentes

1. Canal de privacidade: definir e-mail do encarregado/responsável antes do go-live.
2. Registrar pedidos de titulares, validar identidade e encaminhar ao controlador quando aplicável.
3. Em incidente: conter, preservar evidências, avaliar impacto, comunicar controlador e seguir orientação da ANPD quando cabível.
4. Revisar acessos, backups, fornecedores e políticas ao menos anualmente.
