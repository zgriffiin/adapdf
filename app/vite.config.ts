import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// The page is served by Tauri from `dist` (a custom protocol: no web server, no network).
// `pnpm dev` starts Vite on a fixed port for `pnpm tauri dev`.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { host: "localhost", port: 5173, strictPort: true },
  build: {
    target: "es2022",
    outDir: "dist",
    emptyOutDir: true,
    // Fonts stay as files in `dist/assets`: a data URL would need `data:` in the CSP.
    assetsInlineLimit: 0,
  },
  css: {
    preprocessorOptions: {
      scss: { quietDeps: true, silenceDeprecations: ["import", "global-builtin"] },
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
