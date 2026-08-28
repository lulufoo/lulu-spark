import { cpSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

const host = process.env.TAURI_DEV_HOST;

function copyStaticVendor() {
  return {
    name: 'copy-static-vendor',
    closeBundle() {
      const from = resolve('frontend/vendor');
      const to = resolve('frontend/dist/vendor');
      if (existsSync(from)) {
        cpSync(from, to, { recursive: true });
      }
    },
  };
}

export default defineConfig({
  root: 'frontend',
  base: './',
  plugins: [react(), copyStaticVendor()],
  clearScreen: false,
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: 'ws', host, port: 1421 }
      : undefined,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    target:
      process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'safari13',
    minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
});
