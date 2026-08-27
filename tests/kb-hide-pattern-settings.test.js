// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach } from 'vitest';

import {
  KB_HIDE_PATTERN_KEY,
  getKbHidePattern,
  saveKbHidePattern,
} from '../frontend/js/corpus/kb-hide-pattern.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const indexHtml = readFileSync(join(fixtureRoot, 'frontend/index.html'), 'utf8');
const settingsDialogJs = readFileSync(
  join(fixtureRoot, 'frontend/js/components/modals/settings-dialog.js'),
  'utf8',
);

function installLocalStorageMock() {
  const store = {};
  globalThis.localStorage = {
    getItem(key) {
      return Object.prototype.hasOwnProperty.call(store, key) ? store[key] : null;
    },
    setItem(key, value) {
      store[key] = String(value);
    },
    removeItem(key) {
      delete store[key];
    },
    clear() {
      for (const key of Object.keys(store)) delete store[key];
    },
  };
}

describe('kb hide pattern lives in Settings Knowledge Hidden files', () => {
  it('uses Settings Knowledge Hidden files tab', () => {
    expect(indexHtml).toMatch(/data-panel="knowledge">Knowledge</);
    expect(indexHtml).toMatch(/id="settings-panel-knowledge"/);
    expect(indexHtml).toMatch(/data-tab="hidden"[^>]*>Hidden files</);
    expect(indexHtml).toMatch(/id="settings-kb-hide-pattern"/);
    expect(indexHtml).not.toMatch(/id="sediment-kb-corpus-dialog"/);
  });

  it('shows hint example \\.xxx$', () => {
    expect(indexHtml).toMatch(/\\\.xxx\$|\\\\\.xxx\$/);
  });

  it('Settings open loads saved pattern into the input', () => {
    expect(settingsDialogJs).toMatch(/function syncKbHidePatternInput\(/);
    expect(settingsDialogJs).toMatch(/getKbHidePattern\(\)/);
  });
});

describe('kb hide pattern save', () => {
  beforeEach(() => {
    installLocalStorageMock();
  });

  it('persists valid pattern', () => {
    const result = saveKbHidePattern('\\.xxx$');
    expect(result.ok).toBe(true);
    expect(localStorage.getItem(KB_HIDE_PATTERN_KEY)).toBe('\\.xxx$');
    expect(getKbHidePattern()).toBe('\\.xxx$');
  });

  it('rejects invalid pattern and does not write localStorage', () => {
    localStorage.setItem(KB_HIDE_PATTERN_KEY, '\\.old$');
    const result = saveKbHidePattern('[');
    expect(result.ok).toBe(false);
    expect(result.error).toMatch(/invalid/i);
    expect(localStorage.getItem(KB_HIDE_PATTERN_KEY)).toBe('\\.old$');
  });
});
