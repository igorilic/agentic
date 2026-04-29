import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// https://vitejs.dev/config/
export default defineConfig(({ mode }) => ({
  plugins: [react()],
  // Explicitly fold import.meta.env.DEV so Rollup can tree-shake the
  // StartRunFormInner lazy chunk in production builds. Without this explicit
  // define, `@vitejs/plugin-react` still uses the jsxDEV transform when
  // NODE_ENV is unset — even with `--mode production` — which prevents Rollup
  // from seeing the branch as dead code.
  define:
    mode === "production"
      ? { "import.meta.env.DEV": "false" }
      : undefined,
  // Tauri expects a fixed port, fail if that port is not available
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  // Tells Vite to ignore watching `src-tauri`
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2022",
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: ["./src/__tests__/setup.ts"],
  },
}));
