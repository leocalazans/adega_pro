import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 9003,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**", "**/src-web/**"] },
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2021",
    minify: "esbuild",
    sourcemap: true,
    rollupOptions: {
      input: {
        main: "./index.desktop.html",
      },
    },
  },
  test: {
    include: ["src-desktop/lib/**/*.test.ts"],
    exclude: ["src-desktop/lib/types.ts", "src-desktop/**/*.test.ts"],
  },
});
