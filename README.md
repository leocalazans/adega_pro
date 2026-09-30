# CommerceCTRL

Para continuidade por agentes ou clientes MCP, consulte primeiro
[`docs/MCP.md`](docs/MCP.md). O documento contém o inventário funcional, portões de
teste, baseline e regras para atualizações futuras.

PDV desktop offline-first com Tauri/Rust, SQLite e frontend Svelte 5.

## Frontend Svelte

```powershell
npm --prefix frontend-svelte install
npm --prefix frontend-svelte run check
npm --prefix frontend-svelte run dev
```

O Tauri usa `frontend-svelte/dist` no build de produção e a porta `9003` no desenvolvimento. O frontend React/Next anterior permanece em `src` temporariamente como referência de paridade durante a migração.
