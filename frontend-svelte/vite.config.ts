import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 9003,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: { target: "es2021", minify: "esbuild", sourcemap: true },
  test: {
    coverage: {
      provider: "v8",
      reporter: ["text", "html", "json-summary"],
      reportsDirectory: "coverage",
      include: ["src/lib/**/*.ts"],
      exclude: ["src/lib/types.ts", "src/**/*.test.ts"],
      thresholds: { statements: 60, branches: 50, functions: 80, lines: 55 },
    },
  },
});
