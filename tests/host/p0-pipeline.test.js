import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function read(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

describe('P0 Vite + React + TypeScript pipeline', () => {
  it('package.json has Vite / React / TypeScript scripts and deps', () => {
    const pkg = JSON.parse(read('package.json'));
    expect(pkg.scripts.dev).toBe('vite');
    expect(pkg.scripts.build).toMatch(/vite build/);
    expect(pkg.dependencies.react).toBeTruthy();
    expect(pkg.dependencies['react-dom']).toBeTruthy();
    expect(pkg.dependencies.marked).toBeTruthy();
    expect(pkg.dependencies.turndown).toBeTruthy();
    expect(pkg.devDependencies.vite).toBeTruthy();
    expect(pkg.devDependencies.typescript).toBeTruthy();
    expect(pkg.devDependencies['@vitejs/plugin-react']).toBeTruthy();
  });

  it('tauri.conf.json hooks Vite on 5173 and ships frontend/dist', () => {
    const conf = JSON.parse(read('src-tauri/tauri.conf.json'));
    expect(conf.build.beforeDevCommand).toBe('npm run dev');
    expect(conf.build.beforeBuildCommand).toBe('npm run build');
    expect(conf.build.devUrl).toBe('http://localhost:5173');
    expect(conf.build.frontendDist).toBe('../frontend/dist');
    const win = conf.app.windows[0];
    expect(win.titleBarStyle).toBe('Overlay');
    expect(win.hiddenTitle).toBe(true);
    expect(win.trafficLightPosition).toEqual({ x: 16, y: 18 });
  });

  it('index.html enters through src/main.tsx', () => {
    const html = read('frontend/index.html');
    expect(html).toMatch(/src="src\/main\.tsx"/);
    expect(html).toMatch(/id="root"/);
    expect(html).not.toMatch(/src="js\/main\.js"/);
    expect(html).not.toMatch(/app\.css\?v=/);
    expect(html).not.toMatch(/cdn\.jsdelivr\.net/);
    expect(html).not.toMatch(/id="btn-nav-home"/);
    expect(existsSync(join(repoRoot, 'frontend/src/main.tsx'))).toBe(true);
    expect(read('frontend/src/main.tsx')).toMatch(/import\('\.\/boot\.ts'\)/);
    expect(read('frontend/src/main.tsx')).not.toMatch(/StrictMode/);
  });

  it('app wires hash routing through HashRouter in App, not initRouter', () => {
    const main = read('frontend/src/boot.ts');
    expect(main).toMatch(/setRouteHandlers\s*\(/);
    expect(main).not.toMatch(/\binitRouter\s*\(/);
    expect(read('frontend/src/App.tsx')).toMatch(/HashRouter/);
    expect(read('frontend/src/hash-router.tsx')).not.toMatch(/StrictMode/);
    expect(read('frontend/src/hash-router.tsx')).not.toMatch(/\binitRouter\b/);
  });

  it('toast.tsx exports the React showToast', () => {
    expect(read('frontend/src/toast.tsx')).toMatch(/export\s+function\s+showToast\s*\(/);
  });

  it('fetch transport treats Vite 5173 as the Tauri-dev browser warning port', () => {
    const src = read('frontend/src/host/api/transport.ts');
    expect(src).toMatch(/port === ['"]1430['"]/);
    expect(src).toMatch(/port === ['"]5173['"]/);
  });

  it('frontend/js is gone; chrome lives in src', () => {
    expect(existsSync(join(repoRoot, 'frontend/js'))).toBe(false);
    expect(existsSync(join(repoRoot, 'frontend/src/boot.ts'))).toBe(true);
    expect(existsSync(join(repoRoot, 'frontend/src/App.tsx'))).toBe(true);
    expect(existsSync(join(repoRoot, 'frontend/src/shell-html.ts'))).toBe(true);
  });

  it('vite.config copies vendor into dist and uses a relative base', () => {
    const src = read('vite.config.js');
    expect(src).toMatch(/base:\s*['"]\.\/['"]/);
    expect(src).toMatch(/copy-static-vendor|frontend\/vendor/);
  });
});
