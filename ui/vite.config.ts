import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  // Vite resolves the "@/" paths that tsconfig.json declares.
  resolve: { tsconfigPaths: true },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    // A source map would double the committed bundle (EMB-3).
    sourcemap: false,
    // The size step enforces the project's budget (APP-3). This limit only
    // keeps Vite's own fixed warning out of the output.
    chunkSizeWarningLimit: 4096,
    // EMB-2 and EMB-4: fixed file names, so the binary names each file and a
    // rebuild of the same source writes the same bytes. A project that does
    // not adopt `embedded` deletes this block, so Vite puts a content hash in
    // each name and a browser can cache the files.
    rolldownOptions: {
      output: {
        entryFileNames: "assets/app.js",
        chunkFileNames: "assets/[name].js",
        assetFileNames: "assets/[name][extname]",
      },
    },
  },
  // `pnpm dev` sends API requests to the program that serves them (EMB-6).
  server: { proxy: { "/api": "http://localhost:8080" } },
  // The acceptance tests run against `vite preview`, so it sends the
  // Content-Security-Policy that the embedding program sends (EMB-6).
  preview: { headers: { "Content-Security-Policy": "default-src 'self'" } },
});
