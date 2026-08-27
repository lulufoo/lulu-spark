// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, vi } from 'vitest';

vi.mock('../frontend/js/host/api.js', () => ({
  fetchConfig: vi.fn(),
  setConfig: vi.fn(),
  inferGithubUserUrl: vi.fn(),
  checkWorkbenchKnowledgeRoot: vi.fn(),
}));

import * as api from '../frontend/js/host/api.js';
import {
  ENGINE_CATEGORIES,
  listEngineCategories,
  getEnginePreset,
} from '../frontend/js/components/modals/engine-presets.js';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const indexHtml = readFileSync(join(root, 'frontend/index.html'), 'utf8');

function mountSettingsDom() {
  const markup = indexHtml.match(
    /<!-- Settings dialog -->[\s\S]*?<!-- Skills dialog -->/,
  )?.[0];
  if (!markup) throw new Error('settings dialog markup not found');
  document.body.innerHTML = markup.replace('<!-- Skills dialog -->', '');
}

function baseConfig(overrides = {}) {
  return {
    workbench_knowledge_root: '',
    knowledge_corpus_root: '',
    github_user_url: '',
    workbench_github_repo_url: '',
    has_github_token: false,
    assistant_engine: 'host',
    has_host_key: false,
    llm: {
      platform: 'glm',
      base_url: 'https://open.bigmodel.cn/api/paas/v4',
      model: '',
    },
    ...overrides,
  };
}

describe('Host-only Assistant settings', () => {
  it('exposes only Agent Loop / GLM in the preset catalog', () => {
    expect(ENGINE_CATEGORIES).toEqual([
      { id: 'host', label: 'Agent Loop / GLM' },
    ]);
    expect(listEngineCategories()).toEqual([
      { id: 'host', label: 'Agent Loop / GLM' },
    ]);

    const preset = getEnginePreset('host');
    expect(preset.displayName).toBe('Agent Loop / GLM');
    expect(preset.editableFields).toEqual(['model']);
    expect(preset.fields).toEqual({
      platform: 'glm',
      base_url: 'https://open.bigmodel.cn/api/paas/v4',
      model: '',
    });
    expect(getEnginePreset('cursor')).toBeNull();
  });

  it('renders one engine option and no Cursor UI', () => {
    const select = indexHtml.match(
      /<select id="settings-llm-engine">([\s\S]*?)<\/select>/,
    )?.[1];
    expect(select).toContain('value="host"');
    expect(select).not.toMatch(/cursor/i);
    expect(indexHtml).not.toMatch(/Cursor Agent/);
  });

  beforeEach(async () => {
    vi.resetModules();
    vi.clearAllMocks();
    mountSettingsDom();
    api.fetchConfig.mockResolvedValue(baseConfig({
      has_host_key: true,
      llm: {
        platform: 'glm',
        base_url: 'https://open.bigmodel.cn/api/paas/v4',
        model: 'glm-4',
      },
    }));
    api.setConfig.mockResolvedValue(baseConfig({
      has_host_key: true,
      llm: {
        platform: 'glm',
        base_url: 'https://open.bigmodel.cn/api/paas/v4',
        model: 'glm-4',
      },
    }));
    await import('../frontend/js/components/modals/settings-dialog.js');
  });

  it('loads the GLM preset and model without a second engine slot', async () => {
    const { openSettingsDialog } = await import(
      '../frontend/js/components/modals/settings-dialog.js'
    );
    await openSettingsDialog();

    expect(document.getElementById('settings-llm-engine').value).toBe('host');
    expect(document.getElementById('settings-llm-platform').value).toBe('glm');
    expect(document.getElementById('settings-llm-base-url').value).toBe(
      'https://open.bigmodel.cn/api/paas/v4',
    );
    expect(document.getElementById('settings-llm-model').value).toBe('glm-4');
    expect(document.getElementById('settings-llm-key-hint').textContent).toMatch(
      /configured|API key/i,
    );
  });

  it('saves only Host/GLM model and credential fields', async () => {
    const { openSettingsDialog } = await import(
      '../frontend/js/components/modals/settings-dialog.js'
    );
    await openSettingsDialog();
    document.getElementById('settings-llm-model').value = 'glm-4-air';
    document.getElementById('settings-llm-api-key').value = 'sk-host-new';
    document.getElementById('btn-settings-save-llm').click();

    await vi.waitFor(() => expect(api.setConfig).toHaveBeenCalled());
    const payload = api.setConfig.mock.calls[0][0];
    expect(payload).toEqual({
      assistant_engine: 'host',
      llm: { model: 'glm-4-air' },
      api_key_host: 'sk-host-new',
    });
    expect(payload.api_key_cursor).toBeUndefined();
  });

  it.each(['cursor', '', '   ', 'bogus', null, undefined])(
    'treats invalid assistant_engine response %j as the unconfigured Host panel',
    async (assistantEngine) => {
      api.fetchConfig.mockResolvedValueOnce(baseConfig({
        assistant_engine: assistantEngine,
        llm: { platform: '', base_url: '', model: '' },
      }));
      const { openSettingsDialog } = await import(
        '../frontend/js/components/modals/settings-dialog.js'
      );
      await openSettingsDialog();

      expect(document.getElementById('settings-llm-engine').value).toBe('host');
      expect(document.getElementById('settings-llm-model').value).toBe('');
      expect(document.getElementById('settings-llm-platform').value).toBe('glm');
      expect(document.getElementById('settings-llm-base-url').value).toBe(
        'https://open.bigmodel.cn/api/paas/v4',
      );
      expect(document.querySelectorAll('#settings-llm-engine option')).toHaveLength(1);
    },
  );
});
