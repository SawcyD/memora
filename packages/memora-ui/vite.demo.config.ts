import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "node:path";

export default defineConfig({
  root: resolve(__dirname, "demo"),
  plugins: [react()],
  server: { port: 4178, strictPort: true },
  build: { outDir: resolve(__dirname, "demo-dist"), emptyOutDir: true },
});
