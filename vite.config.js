import { cpSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

const host = process.env.TAURI_DEV_HOST;
const THEME_SHEET_LINK =
  '<link rel="stylesheet" id="theme-sheet" href="./theme-light.css" data-theme-id="light">';

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

function keepThemeSheets() {
  const files = ['theme-light.css', 'theme-dark.css'];
  return {
    name: 'keep-theme-sheets',
    apply: 'build',
    transformIndexHtml: {
      order: 'pre',
      handler(html) {
        return html.replace(
          /<link rel="stylesheet" id="theme-sheet"[^>]*>\s*/u,
          '',
        );
      },
    },
    closeBundle() {
      for (const file of files) {
        cpSync(resolve('frontend', file), resolve('frontend/dist', file));
      }
    },
  };
}

function restoreThemeSheet() {
  return {
    name: 'restore-theme-sheet',
    apply: 'build',
    transformIndexHtml: {
      order: 'post',
      handler(html) {
        if (html.includes('id="theme-sheet"')) return html;
        return html.replace(
          /<link rel="stylesheet" crossorigin href="\.\/assets\/[^"]+\.css">/u,
          `${THEME_SHEET_LINK}\n  $&`,
        );
      },
    },
  };
}

export default defineConfig({
  root: 'frontend',
  base: './',
  plugins: [react(), copyStaticVendor(), keepThemeSheets(), restoreThemeSheet()],
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
