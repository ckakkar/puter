import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
export default defineConfig({
  root: 'web',
  plugins: [svelte()],
  build: { outDir: '../dist', emptyOutDir: true },
  server: { port: 5173, strictPort: true },
});
