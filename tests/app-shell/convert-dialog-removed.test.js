import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { listFrontendSourceFiles } from '../helpers/read-frontend-js.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const convertUiPath = join(repoRoot, 'frontend/src/app-shell/ui/convert-dialog.tsx');
const convertStatePath = join(repoRoot, 'frontend/src/app-shell/state/convert.ts');
const bindDialogPath = join(repoRoot, 'frontend/src/app-shell/ui/bind-dialog.tsx');

const OPEN_BIND_DIALOG_SRC = `export async function openBindDialog() {
  logBindEvent('issue_bind', 'started');
  stopPolling();
  stopCountdown();
  bindOpenStore.set(true);
  el('bind-dialog')?.classList.add('open');
  setBindState('loading');
  const status = el('bind-status');
  if (status) status.textContent = 'Loading…';
  const countdown = el('bind-countdown');
  if (countdown) countdown.textContent = '';
  const refresh = el('btn-bind-refresh');
  if (refresh) refresh.hidden = true;
  setHost(null);
  setFooterHint('A new code is available if this one expires.');
  clearPreview();
  try {
    const payload = (await api.invoke('issue_bind')) as BindPayload;
    setHost(payload);
    if (status) status.textContent = 'Waiting for the Android app';
    setBindState('waiting');
    startCountdown(payload.exp ?? 0);
    startPolling();
    logBindEvent('issue_bind', 'succeeded');
    drawPayload(payload);
  } catch (error) {
    const code = bindErrorCode(error);
    logBindEvent('issue_bind', code);
    clearPreview();
    setHost(null);
    setBindState('error');
    if (countdown) countdown.textContent = '';
    if (status) status.textContent = ISSUE_ERROR_STATUS[code] || 'Unable to issue a binding code.';
    if (refresh) refresh.hidden = false;
    setFooterHint('Fix the issue below, then issue a new code.');
  } finally {
    if (el('bind-status')?.textContent === 'Loading…') {
      const late = el('bind-status');
      if (late) late.textContent = '';
    }
  }
}
`;

describe('Convert dialog and convertStore removed', () => {
  it('deletes convert-dialog.tsx and convert.ts', () => {
    expect(existsSync(convertUiPath)).toBe(false);
    expect(existsSync(convertStatePath)).toBe(false);
  });

  it('does not leave ConvertDialog, convertStore, or re-exports', () => {
    const product = listFrontendSourceFiles(join(repoRoot, 'frontend/src'))
      .map((abs) => readFileSync(abs, 'utf8'))
      .join('\n');
    expect(product).not.toMatch(/\bConvertDialog\b/);
    expect(product).not.toMatch(/\bconvertStore\b/);
    expect(product).not.toMatch(/export\s+(?:async\s+)?(?:function|const|class|type)\s+ConvertDialog\b/);
    expect(product).not.toMatch(/export\s+\{[^}]*\bConvertDialog\b/);
    expect(product).not.toMatch(/export\s+(?:async\s+)?(?:function|const|class|type)\s+convertStore\b/);
    expect(product).not.toMatch(/export\s+\{[^}]*\bconvertStore\b/);
  });

  it('does not delete bind-dialog.tsx or change openBindDialog', () => {
    expect(existsSync(bindDialogPath)).toBe(true);
    const src = readFileSync(join(repoRoot, 'frontend/src/app-shell/commands/bind-dialog.ts'), 'utf8');
    expect(src).toContain(OPEN_BIND_DIALOG_SRC);
  });

  it('leaves qrcode.min.js in place', () => {
    expect(existsSync(join(repoRoot, 'frontend/vendor/qrcode.min.js'))).toBe(true);
  });
});
