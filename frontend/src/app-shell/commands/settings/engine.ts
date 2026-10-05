import * as api from '../../../host/api.ts';
import { errMessage, type SettingsConfig } from '../../state/types.ts';
import { getEnginePreset, listEngineCategories } from '../../state/settings/engine-presets.ts';
import {
  DEFAULT_ENGINE_CATEGORY,
  engineBaseUrlByCategory,
  engineKeyHints,
  engineModelByCategory,
  setResult,
  publishEngineCategory,
  store,
} from '../../state/settings/store.ts';

function normalizeEngineCategory(raw: unknown): string {
  const id = String(raw || '').trim();
  return getEnginePreset(id) ? id : DEFAULT_ENGINE_CATEGORY;
}

function ensureEngineCategoryOptions() {
  const select = document.getElementById('settings-llm-engine') as HTMLSelectElement | null;
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

function credentialHintForCategory(_categoryId: string) {
  return engineKeyHints.has_host_key
    ? 'API key configured. Enter a new key to replace it.'
    : 'No API key configured.';
}

function resolveEnginePreset(categoryId: string) {
  return getEnginePreset(categoryId) || getEnginePreset(DEFAULT_ENGINE_CATEGORY);
}

function fillPresetFields(
  categoryId: string,
  opts: { baseUrl?: string; forceBaseUrl?: boolean } = {},
) {
  const preset = resolveEnginePreset(categoryId);
  const platformInput = document.getElementById('settings-llm-platform') as HTMLInputElement | null;
  const baseUrlInput = document.getElementById('settings-llm-base-url') as HTMLInputElement | null;
  if (platformInput) {
    platformInput.value = preset?.fields?.platform ?? '';
    platformInput.readOnly = true;
    platformInput.classList.add('settings-input-readonly');
  }
  if (baseUrlInput) {
    const saved = (opts.baseUrl ?? '').trim();
    const fallback = preset?.fields?.base_url ?? '';
    if (opts.forceBaseUrl || !baseUrlInput.value.trim()) {
      baseUrlInput.value = saved || fallback;
    }
    baseUrlInput.readOnly = false;
    baseUrlInput.classList.remove('settings-input-readonly');
  }
}

/**
 * Fill Agent/LLM panel from config (category, preset, model, base URL, credential hint).
 */
export function loadAssistantEnginePanel(cfg?: SettingsConfig | Record<string, unknown>) {
  ensureEngineCategoryOptions();
  const rec = (cfg ?? {}) as SettingsConfig;
  const categoryId = normalizeEngineCategory(rec.assistant_engine);
  const llm = rec.llm ?? {};
  const preset = resolveEnginePreset(categoryId);

  engineKeyHints.has_host_key = Boolean(rec.has_host_key);
  const engineSelect = document.getElementById('settings-llm-engine') as HTMLSelectElement | null;
  const modelInput = document.getElementById('settings-llm-model') as HTMLInputElement | null;
  const keyHint = document.getElementById('settings-llm-key-hint');
  const apiKeyInput = document.getElementById('settings-llm-api-key') as HTMLInputElement | null;

  store.activeEngineCategory = categoryId;
  publishEngineCategory(categoryId);
  const savedModel = typeof llm.model === 'string' ? llm.model : '';
  engineModelByCategory[categoryId] = savedModel || preset?.fields?.model || '';
  engineBaseUrlByCategory[categoryId] =
    typeof llm.base_url === 'string' && llm.base_url.trim()
      ? llm.base_url.trim()
      : preset?.fields?.base_url || '';

  if (engineSelect) engineSelect.value = categoryId;
  fillPresetFields(categoryId, {
    baseUrl: engineBaseUrlByCategory[categoryId],
    forceBaseUrl: true,
  });
  if (modelInput) {
    modelInput.value = engineModelByCategory[categoryId] ?? '';
    modelInput.readOnly = false;
    modelInput.disabled = false;
  }
  if (apiKeyInput) apiKeyInput.value = '';
  if (keyHint) keyHint.textContent = credentialHintForCategory(categoryId);
}

/**
 * Rebind panel to the selected category preset and any in-session draft.
 */
export async function applyEngineCategorySelection(
  categoryId: unknown,
  { clearCredential = true }: { clearCredential?: boolean } = {},
) {
  const id = normalizeEngineCategory(categoryId);
  const prev = store.activeEngineCategory;
  const engineSelect = document.getElementById('settings-llm-engine') as HTMLSelectElement | null;
  const modelInput = document.getElementById('settings-llm-model') as HTMLInputElement | null;
  const baseUrlInput = document.getElementById('settings-llm-base-url') as HTMLInputElement | null;
  const keyHint = document.getElementById('settings-llm-key-hint');
  const apiKeyInput = document.getElementById('settings-llm-api-key') as HTMLInputElement | null;
  const preset = resolveEnginePreset(id);

  if (modelInput) {
    engineModelByCategory[prev] = modelInput.value;
  }
  if (baseUrlInput) {
    engineBaseUrlByCategory[prev] = baseUrlInput.value;
  }
  store.activeEngineCategory = id;
  publishEngineCategory(id);

  if (engineSelect) engineSelect.value = id;
  fillPresetFields(id, {
    baseUrl: engineBaseUrlByCategory[id] ?? '',
    forceBaseUrl: true,
  });
  if (clearCredential && apiKeyInput) apiKeyInput.value = '';
  if (keyHint) keyHint.textContent = credentialHintForCategory(id);

  if (modelInput && store.activeEngineCategory === id) {
    const draft = engineModelByCategory[id];
    modelInput.value = draft ?? preset?.fields?.model ?? '';
    modelInput.readOnly = false;
    modelInput.disabled = false;
  }
}

export async function saveAssistantEnginePanel() {
  const btn = document.getElementById('btn-settings-save-llm') as HTMLButtonElement;
  const categoryId = normalizeEngineCategory(
    (document.getElementById('settings-llm-engine') as HTMLSelectElement | null)?.value,
  );
  const preset = resolveEnginePreset(categoryId);
  const model =
    (document.getElementById('settings-llm-model') as HTMLInputElement | null)?.value.trim()
    || preset?.fields?.model
    || '';
  const baseUrl =
    (document.getElementById('settings-llm-base-url') as HTMLInputElement | null)?.value.trim()
    || preset?.fields?.base_url
    || '';
  const apiKey = (document.getElementById('settings-llm-api-key') as HTMLInputElement | null)?.value.trim() ?? '';

  const payload: Record<string, unknown> = {
    assistant_engine: categoryId,
    llm: { model, base_url: baseUrl },
  };
  if (apiKey) {
    payload.api_key_host = apiKey;
  }

  btn.disabled = true;
  btn.textContent = 'Saving…';
  try {
    const resp = await api.setConfig(payload);
    if (resp?.error) throw new Error(resp.error);
    const parts = ['LLM', 'Base URL', 'Model'];
    if (apiKey) parts.push('Credential');
    setResult('settings-result-llm', `Saved: ${parts.join(', ')}.`);
    const cleared = document.getElementById('settings-llm-api-key') as HTMLInputElement | null;
    if (cleared) cleared.value = '';
    const { loadSettingsSnapshot } = await import('./snapshot.ts');
    await loadSettingsSnapshot();
  } catch (e) {
    setResult('settings-result-llm', `Save failed: ${errMessage(e, String(e))}`, true);
  } finally {
    btn.disabled = false;
    btn.textContent = 'Save';
  }
}
