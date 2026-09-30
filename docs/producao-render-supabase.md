# Produção: Render, Supabase e atualizador

1. Aplique `render.yaml` no Render e aponte `DATABASE_URL` para o PostgreSQL do Supabase.
2. Crie buckets públicos `product-images` e `desktop-releases` no Supabase Storage.
3. Cadastre no Render todas as variáveis marcadas `sync: false`.
4. Cadastre no GitHub Actions: `TAURI_SIGNING_PRIVATE_KEY`,
   `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `SUPABASE_URL`,
   `SUPABASE_SERVICE_ROLE_KEY`, `COMMERCECTRL_API_URL` e
   `RENDER_DEPLOY_HOOK_URL`.
5. Execute o workflow `Release Windows signed updater`, informando uma versão maior.

O workflow valida o frontend, gera instaladores e artefatos assinados, publica o
pacote e `latest.json` no Supabase Storage e aciona o deploy hook do Render.
A service-role jamais deve ser colocada no frontend ou no executável.

Referências oficiais: Render Deploy Hooks e Environment Variables; Supabase
Storage Standard Uploads; Tauri Updater v2.
