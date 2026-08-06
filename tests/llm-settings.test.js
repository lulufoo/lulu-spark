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
import {
  ENGINE_CATEGORIES,
  listEngineCategories,
  getEnginePreset,
} from '../frontend/js/components/modals/engine-presets.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const indexHtml = readFileSync(join(fixtureRoot, 'frontend/index.html'), 'utf8');

const FORBIDDEN_EDITABLE_KEYS = [
  'cwd',
  'local.cwd',
  'mcpServers',
  'mcp_servers',
  'mcp',
  'cursorSdk',
  'cursor_sdk',
  'sdk',
];

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

function extractLlmPanelMarkup() {
  const match = indexHtml.match(
    /<div id="settings-panel-llm"[\s\S]*?(?=<div id="settings-panel-|<\/div>\s*<\/div>\s*<div id="settings-dialog-footer")/,
  );
  if (!match) throw new Error('settings-panel-llm markup not found');
  return match[0];
}

function navLabelForPanel(panelId) {
  const re = new RegExp(
    `<button[^>]*data-panel="${panelId}"[^>]*>([^<]+)</button>`,
  );
  const match = indexHtml.match(re);
  return match ? match[1].trim() : null;
}

function panelTitleText() {
  const panel = extractLlmPanelMarkup();
  const match = panel.match(
    /<div class="settings-panel-title">([^<]+)<\/div>/,
  );
  return match ? match[1].trim() : null;
}

describe('settings Assistant/Engine panel IA (index.html)', () => {
  it('keeps data-panel="llm" as the engine settings panel key', () => {
    expect(indexHtml).toMatch(/data-panel="llm"/);
    expect(indexHtml).toMatch(/id="settings-panel-llm"/);
  });

  it('nav label is Assistant / Engine (not bare LLM)', () => {
    const label = navLabelForPanel('llm');
    expect(label).toBeTruthy();
    expect(label).toMatch(/Assistant\s*\/\s*Engine/i);
    expect(label).not.toBe('LLM');
  });

  it('panel title is Assistant / Engine (not bare LLM)', () => {
    const title = panelTitleText();
    expect(title).toBeTruthy();
    expect(title).toMatch(/Assistant\s*\/\s*Engine/i);
    expect(title).not.toMatch(/^LLM\b/);
  });

  it('pins Model as the writable field id and omits MCP/SDK/cwd controls', () => {
    const panel = extractLlmPanelMarkup();
    expect(panel).toMatch(/id="settings-llm-model"/);
    expect(panel).toMatch(/id="settings-llm-engine"/);
    expect(panel).not.toMatch(/id="settings-llm-cwd"/);
    expect(panel).not.toMatch(/id="settings-llm-mcp/i);
    expect(panel).not.toMatch(/cursor[_-]?sdk/i);
    expect(panel).not.toMatch(/mcpServers/i);
  });

  it('retains credential and save controls for follow-on UI tasks', () => {
    expect(indexHtml).toMatch(/id="settings-llm-api-key"/);
    expect(indexHtml).toMatch(/id="btn-settings-save-llm"/);
  });

  it('marks preset-carried platform/base_url as readonly or disabled in markup', () => {
    const panel = extractLlmPanelMarkup();
    expect(panel).toMatch(
      /id="settings-llm-platform"[^>]*(?:readonly|disabled)/i,
    );
    expect(panel).toMatch(
      /id="settings-llm-base-url"[^>]*(?:readonly|disabled)/i,
    );
    expect(panel).not.toMatch(
      /id="settings-llm-model"[^>]*(?:readonly|disabled)/i,
    );
  });
});

describe('engine presets catalog (engine-presets.js)', () => {
  it('exports ENGINE_CATEGORIES with cursor and host ids', () => {
    expect(Array.isArray(ENGINE_CATEGORIES)).toBe(true);
    const ids = ENGINE_CATEGORIES.map((c) => c.id).sort();
    expect(ids).toEqual(['cursor', 'host']);
  });

  it('listEngineCategories returns Cursor Agent and Host/GLM labels in English', () => {
    const cats = listEngineCategories();
    expect(cats).toHaveLength(2);
    const byId = Object.fromEntries(cats.map((c) => [c.id, c]));
    expect(byId.cursor.label).toMatch(/Cursor\s*Agent/i);
    expect(byId.host.label).toMatch(/Host|GLM/i);
    for (const c of cats) {
      expect(c.label).toMatch(/^[A-Za-z0-9 /().+-]+$/);
    }
  });

  it('getEnginePreset returns built-in metadata with Model editable and other fields readonly', () => {
    for (const id of ['cursor', 'host']) {
      const preset = getEnginePreset(id);
      expect(preset).toBeTruthy();
      expect(preset.categoryId).toBe(id);
      expect(typeof preset.displayName).toBe('string');
      expect(preset.displayName.length).toBeGreaterThan(0);
      expect(preset.editableFields).toEqual(['model']);
      expect(Array.isArray(preset.readonlyFields)).toBe(true);
      expect(preset.readonlyFields).not.toContain('model');
      expect(preset.readonlyFields.length).toBeGreaterThan(0);
      expect(preset.fields).toBeTruthy();
      expect(typeof preset.fields).toBe('object');
      for (const key of preset.readonlyFields) {
        expect(Object.prototype.hasOwnProperty.call(preset.fields, key)).toBe(
          true,
        );
      }
    }
  });

  it('getEnginePreset returns null for unknown category id (no fabricated MCP/SDK fields)', () => {
    const missing = getEnginePreset('bogus-engine');
    expect(missing).toBeNull();
  });

  it('catalog exports omit editable cwd / Cursor SDK / MCP server config keys', () => {
    for (const id of ['cursor', 'host']) {
      const preset = getEnginePreset(id);
      const keys = new Set([
        ...preset.editableFields,
        ...preset.readonlyFields,
        ...Object.keys(preset.fields),
      ]);
      for (const forbidden of FORBIDDEN_EDITABLE_KEYS) {
        expect(keys.has(forbidden)).toBe(false);
      }
    }
  });

  it('is a read-only data source (no runtime add/remove preset API)', async () => {
    const mod = await import(
      '../frontend/js/components/modals/engine-presets.js'
    );
    expect(typeof mod.addEnginePreset).toBe('undefined');
    expect(typeof mod.removeEnginePreset).toBe('undefined');
    expect(typeof mod.createEnginePreset).toBe('undefined');
    expect(typeof mod.deleteEnginePreset).toBe('undefined');
  });
});

describe('settings Assistant/Engine panel save/load', () => {
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
      assistant_engine: 'host',
      has_llm_key: true,
      has_host_key: true,
      has_cursor_key: false,
      llm: {
        platform: 'glm',
        base_url: 'https://open.bigmodel.cn/api/paas/v4',
        model: 'glm-4',
      },
    });
    api.setConfig.mockResolvedValue({
      assistant_engine: 'host',
      has_host_key: true,
      has_cursor_key: false,
      llm: {
        model: 'glm-4',
      },
    });
    await import('../frontend/js/components/modals/settings-dialog.js');
  });

  it('loads category, readonly preset fields, model, and credential hint on open', async () => {
    const { openSettingsDialog } = await import(
      '../frontend/js/components/modals/settings-dialog.js'
    );
    await openSettingsDialog();
    expect(document.getElementById('settings-llm-engine').value).toBe('host');
    const hostPreset = getEnginePreset('host');
    expect(document.getElementById('settings-llm-platform').value).toBe(
      hostPreset.fields.platform,
    );
    expect(document.getElementById('settings-llm-base-url').value).toBe(
      hostPreset.fields.base_url,
    );
    expect(document.getElementById('settings-llm-model').value).toBe('glm-4');
    expect(document.getElementById('settings-llm-api-key').value).toBe('');
    expect(document.getElementById('settings-llm-key-hint').textContent).toMatch(
      /configured|API key/i,
    );
  });

  it('keeps preset fields readonly/disabled so users cannot rewrite them', async () => {
    const { openSettingsDialog } = await import(
      '../frontend/js/components/modals/settings-dialog.js'
    );
    await openSettingsDialog();
    const platform = document.getElementById('settings-llm-platform');
    const baseUrl = document.getElementById('settings-llm-base-url');
    const model = document.getElementById('settings-llm-model');
    expect(platform.readOnly || platform.disabled).toBe(true);
    expect(baseUrl.readOnly || baseUrl.disabled).toBe(true);
    expect(model.readOnly || model.disabled).toBe(false);
  });

  it('switches credential hint and readonly preset when category changes', async () => {
    const { openSettingsDialog } = await import(
      '../frontend/js/components/modals/settings-dialog.js'
    );
    await openSettingsDialog();
    const engineSelect = document.getElementById('settings-llm-engine');
    engineSelect.value = 'cursor';
    engineSelect.dispatchEvent(new Event('change', { bubbles: true }));
    const cursorPreset = getEnginePreset('cursor');
    expect(document.getElementById('settings-llm-platform').value).toBe(
      cursorPreset.fields.platform,
    );
    expect(document.getElementById('settings-llm-base-url').value).toBe(
      cursorPreset.fields.base_url,
    );
    expect(document.getElementById('settings-llm-api-key').value).toBe('');
    expect(document.getElementById('settings-llm-key-hint').textContent).toMatch(
      /No .*key configured|not configured/i,
    );
    const credentialInputs = document.querySelectorAll(
      '#settings-panel-llm input[type="password"]',
    );
    expect(credentialInputs).toHaveLength(1);
  });

  it('saves assistant_engine, model, and per-category credential without writable preset overrides', async () => {
    const { openSettingsDialog } = await import(
      '../frontend/js/components/modals/settings-dialog.js'
    );
    await openSettingsDialog();
    document.getElementById('settings-llm-engine').value = 'host';
    document.getElementById('settings-llm-model').value = 'glm-4-air';
    document.getElementById('settings-llm-api-key').value = 'sk-host-new';
    document.getElementById('btn-settings-save-llm').click();
    await vi.waitFor(() => expect(api.setConfig).toHaveBeenCalled());
    const payload = api.setConfig.mock.calls[0][0];
    expect(payload.assistant_engine).toBe('host');
    expect(payload.llm).toEqual({ model: 'glm-4-air' });
    expect(payload.api_key_host).toBe('sk-host-new');
    expect(payload.api_key_cursor).toBeUndefined();
    expect(payload.api_key).toBeUndefined();
    expect(payload.llm.platform).toBeUndefined();
    expect(payload.llm.base_url).toBeUndefined();
  });

  it('saves cursor credential under api_key_cursor when category is cursor', async () => {
    const { openSettingsDialog } = await import(
      '../frontend/js/components/modals/settings-dialog.js'
    );
    await openSettingsDialog();
    const engineSelect = document.getElementById('settings-llm-engine');
    engineSelect.value = 'cursor';
    engineSelect.dispatchEvent(new Event('change', { bubbles: true }));
    document.getElementById('settings-llm-model').value = 'composer-1';
    document.getElementById('settings-llm-api-key').value = 'sk-cursor-new';
    document.getElementById('btn-settings-save-llm').click();
    await vi.waitFor(() => expect(api.setConfig).toHaveBeenCalled());
    const payload = api.setConfig.mock.calls[0][0];
    expect(payload.assistant_engine).toBe('cursor');
    expect(payload.llm).toEqual({ model: 'composer-1' });
    expect(payload.api_key_cursor).toBe('sk-cursor-new');
    expect(payload.api_key_host).toBeUndefined();
  });

  it('omits credential keys from payload when credential input is blank', async () => {
    const { openSettingsDialog } = await import(
      '../frontend/js/components/modals/settings-dialog.js'
    );
    await openSettingsDialog();
    document.getElementById('settings-llm-model').value = 'glm-4';
    document.getElementById('settings-llm-api-key').value = '';
    document.getElementById('btn-settings-save-llm').click();
    await vi.waitFor(() => expect(api.setConfig).toHaveBeenCalled());
    const payload = api.setConfig.mock.calls[0][0];
    expect(payload.assistant_engine).toBe('host');
    expect(payload.llm).toEqual({ model: 'glm-4' });
    expect(payload.api_key_host).toBeUndefined();
    expect(payload.api_key_cursor).toBeUndefined();
    expect(payload.api_key).toBeUndefined();
  });

  it('shows English error when setConfig fails and does not pretend success', async () => {
    api.setConfig.mockRejectedValueOnce(new Error('persist denied'));
    const { openSettingsDialog } = await import(
      '../frontend/js/components/modals/settings-dialog.js'
    );
    await openSettingsDialog();
    document.getElementById('settings-llm-model').value = 'glm-4';
    document.getElementById('btn-settings-save-llm').click();
    await vi.waitFor(() => {
      const result = document.getElementById('settings-result-llm').textContent;
      expect(result).toMatch(/fail|error|denied/i);
      expect(result).not.toMatch(/^Saved/i);
    });
    expect(document.getElementById('settings-result-llm').textContent).toMatch(
      /^[A-Za-z0-9 :.,'_\-()/]+$/,
    );
  });

  it('defaults to host and does not throw on empty/missing engine config', async () => {
    api.fetchConfig.mockResolvedValueOnce({
      workbench_knowledge_root: '',
      knowledge_corpus_root: '',
      github_user_url: '',
      has_github_token: false,
    });
    const { openSettingsDialog } = await import(
      '../frontend/js/components/modals/settings-dialog.js'
    );
    await expect(openSettingsDialog()).resolves.toBeUndefined();
    expect(document.getElementById('settings-llm-engine').value).toBe('host');
    const hostPreset = getEnginePreset('host');
    expect(document.getElementById('settings-llm-platform').value).toBe(
      hostPreset.fields.platform,
    );
    expect(document.getElementById('settings-llm-model').value).toBe('');
  });

  it('does not render MCP or Cursor SDK/cwd editable controls in the engine panel', async () => {
    const { openSettingsDialog } = await import(
      '../frontend/js/components/modals/settings-dialog.js'
    );
    await openSettingsDialog();
    const panel = document.getElementById('settings-panel-llm');
    expect(panel.querySelector('#settings-llm-cwd')).toBeNull();
    expect(panel.querySelector('[id*="mcp"]')).toBeNull();
    expect(panel.querySelector('[id*="sdk"]')).toBeNull();
    expect(panel.textContent).not.toMatch(/mcpServers/i);
    expect(panel.textContent).not.toMatch(/Cursor SDK/i);
  });
});
