import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
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
