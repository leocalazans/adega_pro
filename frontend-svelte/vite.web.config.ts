import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { existsSync, renameSync } from "node:fs";
import { resolve } from "node:path";

// O build web usa index.web.html como entrada; o Vercel (e qualquer host
// estático) espera index.html na raiz. Renomeia ao final do bundle.
function webIndexHtml() {
  return {
    name: "commercectrl-web-index",
    apply: "build" as const,
    closeBundle() {
      const dist = resolve(process.cwd(), "dist");
      const entry = resolve(dist, "index.web.html");
      if (existsSync(entry)) {
        renameSync(entry, resolve(dist, "index.html"));
      }
    },
  };
}

export default defineConfig({
  plugins: [svelte(), webIndexHtml()],
  clearScreen: false,
  server: {
    port: 9003,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**", "**/src-desktop/**"] },
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2021",
    minify: "esbuild",
    sourcemap: true,
    rollupOptions: {
      input: {
        main: "./index.web.html",
      },
    },
  },
  test: {
    include: ["src-web/lib/**/*.test.ts"],
    exclude: ["src-web/lib/types.ts", "src-web/**/*.test.ts"],
  },
});
