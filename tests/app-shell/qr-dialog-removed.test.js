import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { listFrontendSourceFiles } from '../helpers/read-frontend-js.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const qrUiPath = join(repoRoot, 'frontend/src/app-shell/ui/qr-dialog.tsx');
const qrCommandPath = join(repoRoot, 'frontend/src/app-shell/commands/qr-dialog.ts');
const bootPath = join(repoRoot, 'frontend/src/boot.ts');
const bindCommandPath = join(repoRoot, 'frontend/src/app-shell/commands/bind-dialog.ts');
const indexHtmlPath = join(repoRoot, 'frontend/index.html');
const qrVendorPath = join(repoRoot, 'frontend/vendor/qrcode.min.js');

const BIND_QR_TO_CANVAS = '  QRCode.toCanvas(canvas, text, QR_OPTS, (err) => {';
const INDEX_QRCODE_SCRIPT = '<script src="vendor/qrcode.min.js"></script>';

describe('QR dialog page removed', () => {
  it('deletes qr-dialog.tsx and qr-dialog.ts', () => {
    expect(existsSync(qrUiPath)).toBe(false);
    expect(existsSync(qrCommandPath)).toBe(false);
  });

  it('does not leave renderQr, openQrDialog, or re-exports', () => {
    const product = listFrontendSourceFiles(join(repoRoot, 'frontend/src'))
      .map((abs) => readFileSync(abs, 'utf8'))
      .join('\n');
    expect(product).not.toMatch(/\brenderQr\b/);
    expect(product).not.toMatch(/\bopenQrDialog\b/);
    expect(product).not.toMatch(
      /export\s+(?:async\s+)?(?:function renderQr|const renderQr|class renderQr|type renderQr)\b/,
    );
    expect(product).not.toMatch(/export\s+\{[^}]*\brenderQr\b/);
    expect(product).not.toMatch(
      /export\s+(?:async\s+)?(?:function openQrDialog|const openQrDialog|class openQrDialog|type openQrDialog)\b/,
    );
    expect(product).not.toMatch(/export\s+\{[^}]*\bopenQrDialog\b/);
  });

  it('drops the boot import of qr-dialog.tsx', () => {
    expect(existsSync(bootPath)).toBe(true);
    const boot = readFileSync(bootPath, 'utf8');
    expect(boot).not.toMatch(/import\s+['"]\.\/app-shell\/ui\/qr-dialog\.tsx['"]/);
    expect(boot).not.toContain('qr-dialog.tsx');
  });

  it('leaves qrcode.min.js and the index.html script tag', () => {
    expect(existsSync(qrVendorPath)).toBe(true);
    const html = readFileSync(indexHtmlPath, 'utf8');
    expect(html).toContain(INDEX_QRCODE_SCRIPT);
  });

  it('does not change the bind-dialog QRCode.toCanvas call', () => {
    expect(existsSync(bindCommandPath)).toBe(true);
    const src = readFileSync(bindCommandPath, 'utf8');
    expect(src).toContain(BIND_QR_TO_CANVAS);
  });
});
