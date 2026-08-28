// @ts-nocheck — settings DOM wiring stays unchecked like checkJs:false.
import * as api from '../../host/api.ts';
import { getEnginePreset, listEngineCategories } from '../engine-presets.ts';
import {
  DEFAULT_ENGINE_CATEGORY,
  engineKeyHints,
  engineModelByCategory,
  setResult,
  store,
} from './store.ts';

function normalizeEngineCategory(raw) {
  const id = String(raw || '').trim();
  return id === 'host' ? id : DEFAULT_ENGINE_CATEGORY;
}

function ensureEngineCategoryOptions() {
  const select = document.getElementById('settings-llm-engine');
  if (!select) return;
  const categories = listEngineCategories();
  const existing = new Set(
    Array.from(select.options).map((opt) => opt.value),
  );
  for (const cat of categories) {
    if (existing.has(cat.id)) continue;
    const opt = document.createElement('option');
    opt.value = cat.id;
    opt.textContent = cat.label;
    select.appendChild(opt);
  }
}

function credentialHintForCategory(categoryId) {
  const hasKey = categoryId === 'host' && engineKeyHints.has_host_key;
  return hasKey
    ? 'API key configured. Enter a new key to replace it.'
    : 'No API key configured.';
}

function resolveEnginePreset(categoryId) {
  return getEnginePreset(categoryId) || getEnginePreset(DEFAULT_ENGINE_CATEGORY);
}

function fillReadonlyPresetFields(categoryId) {
  const preset = resolveEnginePreset(categoryId);
  const platformInput = document.getElementById('settings-llm-platform');
  const baseUrlInput = document.getElementById('settings-llm-base-url');
  if (platformInput) {
    platformInput.value = preset?.fields?.platform ?? '';
    platformInput.readOnly = true;
    platformInput.classList.add('settings-input-readonly');
  }
  if (baseUrlInput) {
    baseUrlInput.value = preset?.fields?.base_url ?? '';
    baseUrlInput.readOnly = true;
    baseUrlInput.classList.add('settings-input-readonly');
  }
}

/**
 * Fill Assistant/Engine panel from config (category, readonly preset, model, credential hint).
 */
export function loadAssistantEnginePanel(cfg) {
  ensureEngineCategoryOptions();
  const categoryId = normalizeEngineCategory(cfg?.assistant_engine);
  const llm = cfg?.llm ?? {};

  engineKeyHints.has_host_key = Boolean(cfg?.has_host_key);
  const engineSelect = document.getElementById('settings-llm-engine');
  const modelInput = document.getElementById('settings-llm-model');
  const keyHint = document.getElementById('settings-llm-key-hint');
  const apiKeyInput = document.getElementById('settings-llm-api-key');

  store.activeEngineCategory = categoryId;
  // The facade exposes only the Host/GLM model.
  engineModelByCategory.host = undefined;
  engineModelByCategory[categoryId] =
    typeof llm.model === 'string' ? llm.model : '';

  if (engineSelect) engineSelect.value = categoryId;
  fillReadonlyPresetFields(categoryId);
  if (modelInput) {
    modelInput.value = engineModelByCategory[categoryId] ?? '';
    modelInput.readOnly = false;
    modelInput.disabled = false;
  }
  if (apiKeyInput) apiKeyInput.value = '';
  if (keyHint) keyHint.textContent = credentialHintForCategory(categoryId);
}

/**
 * Rebind panel to the Host/GLM model + readonly preset fields.
 */
export async function applyEngineCategorySelection(
  categoryId,
  { clearCredential = true } = {},
) {
  const id = normalizeEngineCategory(categoryId);
  const prev = store.activeEngineCategory;
  const engineSelect = document.getElementById('settings-llm-engine');
  const modelInput = document.getElementById('settings-llm-model');
  const keyHint = document.getElementById('settings-llm-key-hint');
  const apiKeyInput = document.getElementById('settings-llm-api-key');

  if (modelInput) {
    engineModelByCategory[prev] = modelInput.value;
  }
  store.activeEngineCategory = id;

  if (engineSelect) engineSelect.value = id;
  fillReadonlyPresetFields(id);
  if (clearCredential && apiKeyInput) apiKeyInput.value = '';
  if (keyHint) keyHint.textContent = credentialHintForCategory(id);

  if (modelInput && store.activeEngineCategory === id) {
    modelInput.value = engineModelByCategory[id] ?? '';
    modelInput.readOnly = false;
    modelInput.disabled = false;
  }
}

export async function saveAssistantEnginePanel() {
  const btn = document.getElementById('btn-settings-save-llm');
  const categoryId = normalizeEngineCategory(
    document.getElementById('settings-llm-engine')?.value,
  );
  const model = document.getElementById('settings-llm-model')?.value.trim() ?? '';
  const apiKey = document.getElementById('settings-llm-api-key')?.value.trim() ?? '';

  const payload = {
    assistant_engine: 'host',
    llm: { model },
  };
  if (apiKey) {
    payload.api_key_host = apiKey;
  }

  btn.disabled = true;
  btn.textContent = 'Saving…';
  try {
    const resp = await api.setConfig(payload);
    if (resp?.error) throw new Error(resp.error);
    const parts = ['Engine', 'Model'];
    if (apiKey) parts.push('Credential');
    setResult('settings-result-llm', `Saved: ${parts.join(', ')}.`);
    document.getElementById('settings-llm-api-key').value = '';
    const { loadSettingsSnapshot } = await import('./snapshot.tsx');
    await loadSettingsSnapshot();
  } catch (e) {
    setResult('settings-result-llm', `Save failed: ${e.message || String(e)}`, true);
  } finally {
    btn.disabled = false;
    btn.textContent = 'Save';
  }
}
