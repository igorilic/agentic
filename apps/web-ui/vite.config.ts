import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// https://vitejs.dev/config/
export default defineConfig(({ mode }) => ({
  plugins: [react()],
  // `@vitejs/plugin-react` emits the jsxDEV (development) transform whenever
  // import.meta.env.DEV is truthy at build time. With `--mode production` the
  // flag is eventually replaced in the output, but the plugin has already
  // chosen the dev transform during the transform phase, before Rollup applies
  // replacements. The lazy-import branch `if (import.meta.env.DEV) { … }`
  // therefore never becomes statically-false, so Rollup cannot tree-shake
  // StartRunFormInner out of the bundle. Explicitly defining the constant to
  // `false` at transform time forces the jsx runtime path and lets Rollup
  // eliminate the dead branch (verified: without this, the chunk leaks into
  // dist/; with it, the grep is empty).
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
