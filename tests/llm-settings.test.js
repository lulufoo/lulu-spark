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

describe('settings LLM panel markup (index.html)', () => {
  it('adds llm nav item with data-panel="llm"', () => {
    expect(indexHtml).toMatch(/data-panel="llm"/);
  });

  it('adds llm panel and editable fields', () => {
    expect(indexHtml).toMatch(/id="settings-panel-llm"/);
    expect(indexHtml).toMatch(/id="settings-llm-platform"/);
    expect(indexHtml).toMatch(/id="settings-llm-base-url"/);
    expect(indexHtml).toMatch(/id="settings-llm-model"/);
    expect(indexHtml).toMatch(/id="settings-llm-api-key"/);
    expect(indexHtml).toMatch(/id="btn-settings-save-llm"/);
  });
});

describe('settings LLM panel save/load', () => {
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
      has_llm_key: true,
      llm: {
        platform: 'kimi',
        base_url: 'https://api.moonshot.cn',
        model: 'moonshot-v1-8k',
      },
    });
    api.setConfig.mockResolvedValue({
      has_llm_key: true,
      llm: {
        platform: 'glm',
        base_url: 'https://open.bigmodel.cn',
        model: 'glm-4',
      },
    });
    await import('../frontend/js/components/modals/settings-dialog.js');
  });

  it('loads llm fields from get_config on open', async () => {
    const { openSettingsDialog } = await import(
      '../frontend/js/components/modals/settings-dialog.js'
    );
    await openSettingsDialog();
    expect(document.getElementById('settings-llm-platform').value).toBe('kimi');
    expect(document.getElementById('settings-llm-base-url').value).toBe(
      'https://api.moonshot.cn',
    );
    expect(document.getElementById('settings-llm-model').value).toBe('moonshot-v1-8k');
    expect(document.getElementById('settings-llm-api-key').value).toBe('');
  });

  it('saves llm fields via setConfig without echoing prior key into payload when blank', async () => {
    const { openSettingsDialog } = await import(
      '../frontend/js/components/modals/settings-dialog.js'
    );
    await openSettingsDialog();
    document.getElementById('settings-llm-platform').value = 'glm';
    document.getElementById('settings-llm-base-url').value = 'https://open.bigmodel.cn';
    document.getElementById('settings-llm-model').value = 'glm-4';
    document.getElementById('settings-llm-api-key').value = 'sk-new';
    document.getElementById('btn-settings-save-llm').click();
    await vi.waitFor(() => expect(api.setConfig).toHaveBeenCalled());
    expect(api.setConfig).toHaveBeenCalledWith({
      llm: {
        platform: 'glm',
        base_url: 'https://open.bigmodel.cn',
        model: 'glm-4',
      },
      api_key: 'sk-new',
    });
  });
});
