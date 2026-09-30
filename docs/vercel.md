# Publicação no Vercel

Configure o diretório raiz do projeto Vercel como `frontend-svelte`. O Vercel
publica a interface estática compilada em `dist`; não hospeda o PDV Tauri,
SQLite, o servidor local de TV nem PostgreSQL.

Antes de publicar o painel do proprietário ou a TV online, hospede o backend
Rust em um serviço com HTTPS e banco PostgreSQL. Use uma URL pública da API em
variáveis `VITE_` apenas para endpoints de leitura pública; chaves de terminal,
Asaas e proprietário nunca entram no Vercel nem no navegador.

A tela online é `https://SEU-DOMINIO/tv/<token>` quando o proxy encaminhar essa
rota ao backend. O token é criado pelo endpoint administrativo e deve ser
tratado como segredo de leitura do canal daquela unidade.
