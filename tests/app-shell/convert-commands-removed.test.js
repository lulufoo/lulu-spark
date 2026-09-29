import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { listFrontendSourceFiles } from '../helpers/read-frontend-js.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const convertCommandPath = join(repoRoot, 'frontend/src/app-shell/commands/convert-dialog.ts');

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

describe('Convert command functions removed', () => {
  it('deletes convert-dialog.ts and the three command functions', () => {
    expect(existsSync(convertCommandPath)).toBe(false);
    const product = listFrontendSourceFiles(join(repoRoot, 'frontend/src'))
      .map((abs) => readFileSync(abs, 'utf8'))
      .join('\n');
    expect(product).not.toMatch(/\bsetConvertTab\b/);
    expect(product).not.toMatch(/\bopenConvertDialog\b/);
    expect(product).not.toMatch(/\bcloseConvertDialog\b/);
  });

  it('does not modify openBindDialog', () => {
    const src = readFileSync(join(repoRoot, 'frontend/src/app-shell/commands/bind-dialog.ts'), 'utf8');
    expect(src).toContain(OPEN_BIND_DIALOG_SRC);
  });
});
