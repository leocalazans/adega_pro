# Produção: Render, Supabase e atualizador

1. Aplique `render.yaml` no Render e aponte `DATABASE_URL` para o PostgreSQL do Supabase.
2. Crie buckets públicos `product-images` e `desktop-releases` no Supabase Storage.
3. Cadastre no Render todas as variáveis marcadas `sync: false`.
4. Cadastre no GitHub Actions: `TAURI_SIGNING_PRIVATE_KEY`,
   `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `COMMERCECTRL_LICENSE_PUBLIC_KEY_B64`,
   `SUPABASE_URL`, `SUPABASE_SERVICE_ROLE_KEY`, `COMMERCECTRL_API_URL` e
   `RENDER_DEPLOY_HOOK_URL`.
5. Execute o workflow `Release Windows signed updater`, informando uma versão maior.

O workflow valida o frontend, gera instaladores e artefatos assinados, publica o
pacote e `latest.json` no Supabase Storage e aciona o deploy hook do Render.
A service-role jamais deve ser colocada no frontend ou no executável.

## Publicação local assistida

Quando o GitHub Actions não estiver disponível, execute `scripts\release-local.cmd`
em um CMD. Ele valida que a chave pública configurada no Tauri corresponde à chave
privada local, pede a senha dessa chave e a Service Role sem exibi-las, gera o
instalador assinado pelo updater e publica o instalador, a assinatura e o
`latest.json`. A chave pública de licenciamento é obtida automaticamente do
backend; ela não é a chave do atualizador e não substitui a chave privada do
backend.

O `.sig` e o `latest.json` atendem à atualização automática do Tauri. Eles não
substituem uma assinatura Authenticode: para remover o aviso de reputação do
Windows, ainda é preciso adquirir/configurar um certificado de code signing.

Referências oficiais: Render Deploy Hooks e Environment Variables; Supabase
Storage Standard Uploads; Tauri Updater v2.
