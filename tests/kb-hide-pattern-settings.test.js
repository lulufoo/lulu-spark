// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, vi } from 'vitest';

vi.mock('../frontend/js/api.js', () => ({
  fetchConfig: vi.fn(),
  setConfig: vi.fn(),
  inferGithubUserUrl: vi.fn(),
  checkWorkbenchKnowledgeRoot: vi.fn(),
}));

import * as api from '../frontend/js/api.js';
import { KB_HIDE_PATTERN_KEY, getKbHidePattern } from '../frontend/js/kb-hide-pattern.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const indexHtml = readFileSync(join(fixtureRoot, 'frontend/index.html'), 'utf8');

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

function mountSettingsDom() {
  const settingsSection = indexHtml.match(
    /<!-- Settings dialog -->[\s\S]*?<!-- Skills dialog -->/,
  )?.[0]?.replace('<!-- Skills dialog -->', '');
  if (!settingsSection) throw new Error('settings dialog markup not found');
  document.body.innerHTML = settingsSection;
}

describe('settings knowledge panel markup (index.html)', () => {
  it('adds knowledge nav item with data-panel="knowledge"', () => {
    expect(indexHtml).toMatch(/data-panel="knowledge"/);
  });

  it('adds kb hide pattern input #settings-kb-hide-pattern', () => {
    expect(indexHtml).toMatch(/id="settings-kb-hide-pattern"/);
  });

  it('shows hint example \\.xxx$', () => {
    expect(indexHtml).toMatch(/\\\.xxx\$|\\\\\.xxx\$/);
  });
});

describe('settings knowledge panel save', () => {
  beforeEach(async () => {
    vi.resetModules();
    vi.clearAllMocks();
    installLocalStorageMock();
    mountSettingsDom();
    api.fetchConfig.mockResolvedValue({
      workbench_knowledge_root: '',
      knowledge_corpus_root: '',
      github_user_url: '',
      has_github_token: false,
    });
    await import('../frontend/js/components/modals/settings-dialog.js');
  });

  it('loads saved pattern from localStorage on open', async () => {
    localStorage.setItem(KB_HIDE_PATTERN_KEY, '\\.xxx$');
    const { openSettingsDialog } = await import(
      '../frontend/js/components/modals/settings-dialog.js'
    );
    await openSettingsDialog();
    expect(document.getElementById('settings-kb-hide-pattern').value).toBe('\\.xxx$');
  });

  it('persists valid pattern via save button', async () => {
    const input = document.getElementById('settings-kb-hide-pattern');
    const btn = document.getElementById('btn-settings-save-knowledge');
    input.value = '\\.xxx$';
    btn.click();
    expect(localStorage.getItem(KB_HIDE_PATTERN_KEY)).toBe('\\.xxx$');
    expect(getKbHidePattern()).toBe('\\.xxx$');
  });

  it('rejects invalid pattern and does not write localStorage', async () => {
    localStorage.setItem(KB_HIDE_PATTERN_KEY, '\\.old$');
    const input = document.getElementById('settings-kb-hide-pattern');
    const btn = document.getElementById('btn-settings-save-knowledge');
    const resultEl = document.getElementById('settings-result-knowledge');
    input.value = '[';
    btn.click();
    expect(localStorage.getItem(KB_HIDE_PATTERN_KEY)).toBe('\\.old$');
    expect(resultEl.textContent).toMatch(/无效|错误|invalid/i);
  });
});
