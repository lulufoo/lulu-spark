// @vitest-environment jsdom
import { createElement } from 'react';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { listFrontendSourceFiles, readShellHtml } from '../helpers/read-frontend-js.js';
import { renderToHtml } from '../../frontend/src/island.ts';
import { SettingsDialog } from '../../frontend/src/app-shell/ui/settings/dialog.tsx';

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchConfig: vi.fn(),
  setConfig: vi.fn(),
}));

import * as api from '../../frontend/src/host/api.ts';
import {
  ENGINE_CATEGORIES,
  listEngineCategories,
  getEnginePreset,
} from '../../frontend/src/app-shell/state/settings/engine-presets.ts';

const root = join(dirname(fileURLToPath(import.meta.url)), '../..');
const appShellSrc = listFrontendSourceFiles(join(root, 'frontend/src/app-shell'))
  .sort()
  .map((abs) => readFileSync(abs, 'utf8'))
  .join('\n');
const indexHtml = [readShellHtml(), appShellSrc].join('\n');

function mountSettingsDom() {
  document.body.innerHTML = renderToHtml(createElement(SettingsDialog));
}

const ENGINE_DEFAULTS = {
  host: { platform: 'glm', base_url: 'https://open.bigmodel.cn/api/paas/v4', model: '' },
  openai: { platform: 'openai', base_url: 'https://api.openai.com/v1', model: '' },
  claude: { platform: 'claude', base_url: 'https://api.anthropic.com/v1', model: '' },
  grok: { platform: 'grok', base_url: 'https://api.x.ai/v1', model: '' },
  kimi: { platform: 'kimi', base_url: 'https://api.moonshot.cn/v1', model: '' },
  qwen: {
    platform: 'qwen',
    base_url: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
    model: '',
  },
};

/** Six-engine `llm` map as get_config returns it; `saved` overrides per engine. */
function llmMap(saved = {}) {
  return Object.fromEntries(
    Object.entries(ENGINE_DEFAULTS).map(([id, fields]) => [
      id,
      { ...fields, key_saved: false, ...saved[id] },
    ]),
  );
}

function baseConfig(overrides = {}) {
  return {
    spark_root: '',
    assistant_engine: 'host',
    llm: llmMap(),
    ...overrides,
  };
}

const SIX_DOTS = '••••••';

describe('Host-only Assistant settings', () => {
  it('exposes OpenAI, Claude, Grok, GLM, Kimi, and Qwen presets', () => {
    expect(ENGINE_CATEGORIES.map((c) => c.id)).toEqual([
      'openai',
      'claude',
      'grok',
      'host',
      'kimi',
      'qwen',
    ]);
    expect(listEngineCategories().map((c) => c.label)).toEqual([
      'OpenAI',
      'Claude',
      'Grok',
      'GLM',
      'Kimi',
      'Qwen',
    ]);

    const glm = getEnginePreset('host');
    expect(glm.displayName).toBe('GLM');
    expect(glm.editableFields).toEqual(['model', 'base_url']);
    expect(glm.fields).toEqual({
      platform: 'glm',
      base_url: 'https://open.bigmodel.cn/api/paas/v4',
      model: '',
    });
    expect(getEnginePreset('openai').fields).toEqual({
      platform: 'openai',
      base_url: 'https://api.openai.com/v1',
      model: 'gpt-4.1',
    });
    expect(getEnginePreset('claude').fields.base_url).toBe('https://api.anthropic.com/v1');
    expect(getEnginePreset('grok').fields.base_url).toBe('https://api.x.ai/v1');
    expect(getEnginePreset('kimi').fields.base_url).toBe('https://api.moonshot.cn/v1');
    expect(getEnginePreset('qwen').fields.base_url).toBe(
      'https://dashscope.aliyuncs.com/compatible-mode/v1',
    );
    expect(getEnginePreset('cursor')).toBeNull();
  });

  it('renders the six category options and no Cursor UI', () => {
    const select = indexHtml.match(
      /<select id="settings-llm-engine">([\s\S]*?)<\/select>/,
    )?.[1];
    expect(select).toContain('value="openai"');
    expect(select).toContain('value="claude"');
    expect(select).toContain('value="grok"');
    expect(select).toContain('value="host"');
    expect(select).toContain('value="kimi"');
    expect(select).toContain('value="qwen"');
    expect(select).not.toMatch(/cursor/i);
    expect(indexHtml).not.toMatch(/Cursor Agent/);
  });

  beforeEach(async () => {
    vi.resetModules();
    vi.clearAllMocks();
    mountSettingsDom();
    api.fetchConfig.mockResolvedValue(baseConfig({
      llm: llmMap({ host: { model: 'glm-4', key_saved: true } }),
    }));
    api.setConfig.mockResolvedValue(baseConfig({
      llm: llmMap({ host: { model: 'glm-4', key_saved: true } }),
    }));
    await import('../../frontend/src/app-shell/commands/settings/dialog.ts');
  });

  it('loads the GLM preset and model among the six categories', async () => {
    const { openSettingsDialog } = await import(
      '../../frontend/src/app-shell/commands/settings/dialog.ts'
    );
    await openSettingsDialog();

    expect(document.getElementById('settings-llm-engine').value).toBe('host');
    expect(document.getElementById('settings-llm-platform').value).toBe('glm');
    expect(document.getElementById('settings-llm-base-url').value).toBe(
      'https://open.bigmodel.cn/api/paas/v4',
    );
    expect(document.getElementById('settings-llm-base-url').readOnly).toBe(false);
    expect(document.getElementById('settings-llm-model').value).toBe('glm-4');
    expect(document.querySelectorAll('#settings-llm-engine option')).toHaveLength(6);
    expect(document.getElementById('settings-llm-key-hint').textContent).toMatch(
      /configured|API key/i,
    );
  });

  it('loads a saved Base URL and keeps the field editable', async () => {
    api.fetchConfig.mockResolvedValueOnce(baseConfig({
      llm: llmMap({ host: { base_url: 'https://proxy.example/v4', model: 'glm-4' } }),
    }));
    const { openSettingsDialog } = await import(
      '../../frontend/src/app-shell/commands/settings/dialog.ts'
    );
    await openSettingsDialog();

    const input = document.getElementById('settings-llm-base-url');
    expect(input.value).toBe('https://proxy.example/v4');
    expect(input.readOnly).toBe(false);
  });

  it('saves only Host/GLM model and credential fields', async () => {
    const { openSettingsDialog } = await import(
      '../../frontend/src/app-shell/commands/settings/dialog.ts'
    );
    await openSettingsDialog();
    document.getElementById('settings-llm-model').value = 'glm-4-air';
    document.getElementById('settings-llm-api-key').value = 'sk-host-new';
    document.getElementById('btn-settings-save-llm').click();

    await vi.waitFor(() => expect(api.setConfig).toHaveBeenCalled());
    const payload = api.setConfig.mock.calls[0][0];
    expect(payload).toEqual({
      assistant_engine: 'host',
      llm: {
        model: 'glm-4-air',
        base_url: 'https://open.bigmodel.cn/api/paas/v4',
      },
      llm_api_key: { engine: 'host', value: 'sk-host-new' },
    });
    expect(payload.api_key_host).toBeUndefined();
    expect(payload.api_key_cursor).toBeUndefined();
  });

  it('saves a custom Base URL', async () => {
    const { openSettingsDialog } = await import(
      '../../frontend/src/app-shell/commands/settings/dialog.ts'
    );
    await openSettingsDialog();
    document.getElementById('settings-llm-base-url').value = 'https://proxy.example/v4';
    document.getElementById('settings-llm-model').value = 'glm-4-air';
    document.getElementById('btn-settings-save-llm').click();

    await vi.waitFor(() => expect(api.setConfig).toHaveBeenCalled());
    expect(api.setConfig.mock.calls[0][0].llm).toEqual({
      model: 'glm-4-air',
      base_url: 'https://proxy.example/v4',
    });
  });

  it.each(['cursor', '', '   ', 'bogus', null, undefined])(
    'treats invalid assistant_engine response %j as the unconfigured Host panel',
    async (assistantEngine) => {
      api.fetchConfig.mockResolvedValueOnce(baseConfig({
        assistant_engine: assistantEngine,
        llm: llmMap(),
      }));
      const { openSettingsDialog } = await import(
        '../../frontend/src/app-shell/commands/settings/dialog.ts'
      );
      await openSettingsDialog();

      expect(document.getElementById('settings-llm-engine').value).toBe('host');
      expect(document.getElementById('settings-llm-model').value).toBe('');
      expect(document.getElementById('settings-llm-platform').value).toBe('glm');
      expect(document.getElementById('settings-llm-base-url').value).toBe(
        'https://open.bigmodel.cn/api/paas/v4',
      );
      expect(document.querySelectorAll('#settings-llm-engine option')).toHaveLength(6);
    },
  );

  it('fills OpenAI preset fields when the category changes', async () => {
    const { openSettingsDialog } = await import(
      '../../frontend/src/app-shell/commands/settings/dialog.ts'
    );
    await openSettingsDialog();
    const select = document.getElementById('settings-llm-engine');
    select.value = 'openai';
    select.dispatchEvent(new Event('change', { bubbles: true }));

    await vi.waitFor(() => {
      expect(document.getElementById('settings-llm-platform').value).toBe('openai');
    });
    expect(document.getElementById('settings-llm-base-url').value).toBe(
      'https://api.openai.com/v1',
    );
    expect(document.getElementById('settings-llm-model').value).toBe('gpt-4.1');

    document.getElementById('btn-settings-save-llm').click();
    await vi.waitFor(() => expect(api.setConfig).toHaveBeenCalled());
    expect(api.setConfig.mock.calls[0][0]).toMatchObject({
      assistant_engine: 'openai',
      llm: {
        model: 'gpt-4.1',
        base_url: 'https://api.openai.com/v1',
      },
    });
  });
  describe('per-category credential', () => {
    async function openPanel() {
      const { openSettingsDialog } = await import(
        '../../frontend/src/app-shell/commands/settings/dialog.ts'
      );
      await openSettingsDialog();
    }

    function selectCategory(id) {
      const select = document.getElementById('settings-llm-engine');
      select.value = id;
      select.dispatchEvent(new Event('change', { bubbles: true }));
    }

    it('shows six dots in a password field for a category with a saved key', async () => {
      await openPanel();
      const keyInput = document.getElementById('settings-llm-api-key');
      expect(keyInput.type).toBe('password');
      expect(keyInput.value).toBe(SIX_DOTS);
      expect(keyInput.value).toHaveLength(6);
      expect(document.getElementById('settings-llm-key-hint').textContent).toBe(
        'API key configured. Enter a new key to replace it.',
      );
    });

    it('leaves the field empty for a category without a saved key', async () => {
      api.fetchConfig.mockResolvedValueOnce(baseConfig({ llm: llmMap() }));
      await openPanel();
      expect(document.getElementById('settings-llm-api-key').value).toBe('');
      expect(document.getElementById('settings-llm-key-hint').textContent).toBe(
        'No API key configured.',
      );
    });

    it('follows the saved state of each category when switching', async () => {
      await openPanel();
      const keyInput = document.getElementById('settings-llm-api-key');
      const hint = document.getElementById('settings-llm-key-hint');

      selectCategory('openai');
      await vi.waitFor(() => {
        expect(document.getElementById('settings-llm-platform').value).toBe('openai');
      });
      expect(keyInput.value).toBe('');
      expect(hint.textContent).toBe('No API key configured.');

      selectCategory('host');
      await vi.waitFor(() => {
        expect(document.getElementById('settings-llm-platform').value).toBe('glm');
      });
      expect(keyInput.value).toBe(SIX_DOTS);
      expect(hint.textContent).toBe('API key configured. Enter a new key to replace it.');
    });

    it('does not send the six dots when the key is left untouched', async () => {
      await openPanel();
      document.getElementById('btn-settings-save-llm').click();

      await vi.waitFor(() => expect(api.setConfig).toHaveBeenCalled());
      expect(api.setConfig.mock.calls[0][0].llm_api_key).toBeUndefined();
    });

    it('empties the dots on focus and restores them on blur when nothing is typed', async () => {
      await openPanel();
      const keyInput = document.getElementById('settings-llm-api-key');

      keyInput.dispatchEvent(new Event('focus'));
      expect(keyInput.value).toBe('');
      keyInput.dispatchEvent(new Event('blur'));
      expect(keyInput.value).toBe(SIX_DOTS);

      keyInput.dispatchEvent(new Event('focus'));
      keyInput.value = 'sk-typed';
      keyInput.dispatchEvent(new Event('blur'));
      expect(keyInput.value).toBe('sk-typed');
    });

    it('sends a typed key for the selected category only', async () => {
      await openPanel();
      selectCategory('openai');
      await vi.waitFor(() => {
        expect(document.getElementById('settings-llm-platform').value).toBe('openai');
      });
      document.getElementById('settings-llm-api-key').value = 'sk-openai-new';
      document.getElementById('btn-settings-save-llm').click();

      await vi.waitFor(() => expect(api.setConfig).toHaveBeenCalled());
      const payload = api.setConfig.mock.calls[0][0];
      expect(payload.assistant_engine).toBe('openai');
      expect(payload.llm_api_key).toEqual({ engine: 'openai', value: 'sk-openai-new' });
    });

    it('shows the saved model and Base URL of the category after switching', async () => {
      api.fetchConfig.mockResolvedValueOnce(baseConfig({
        llm: llmMap({
          host: { model: 'glm-4', key_saved: true },
          openai: {
            base_url: 'https://proxy.openai.example/v1',
            model: 'gpt-custom',
            key_saved: true,
          },
        }),
      }));
      await openPanel();

      selectCategory('openai');
      await vi.waitFor(() => {
        expect(document.getElementById('settings-llm-platform').value).toBe('openai');
      });
      expect(document.getElementById('settings-llm-base-url').value).toBe(
        'https://proxy.openai.example/v1',
      );
      expect(document.getElementById('settings-llm-model').value).toBe('gpt-custom');
      expect(document.getElementById('settings-llm-api-key').value).toBe(SIX_DOTS);
    });

    it('falls back to the preset model when the host has none saved', async () => {
      await openPanel();
      selectCategory('openai');
      await vi.waitFor(() => {
        expect(document.getElementById('settings-llm-platform').value).toBe('openai');
      });
      expect(document.getElementById('settings-llm-model').value).toBe('gpt-4.1');
    });
  });
});
