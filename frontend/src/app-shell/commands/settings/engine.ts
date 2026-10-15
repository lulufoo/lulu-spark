import * as api from '../../../host/api.ts';
import { errMessage, type SettingsConfig } from '../../state/types.ts';
import { getEnginePreset, listEngineCategories } from '../../state/settings/engine-presets.ts';
import { syncSettingsListSelect } from '../../ui/settings/list-select.ts';
import {
  DEFAULT_ENGINE_CATEGORY,
  engineBaseUrlByCategory,
  engineKeySavedByCategory,
  engineModelByCategory,
  SAVED_KEY_PLACEHOLDER,
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

function credentialHintForCategory(categoryId: string) {
  return engineKeySavedByCategory[categoryId]
    ? 'API key configured. Enter a new key to replace it.'
    : 'No API key configured.';
}

/** Saved key → six dots; unsaved → empty. The hint follows the same flag. */
function paintCredentialForCategory(categoryId: string) {
  const apiKeyInput = document.getElementById('settings-llm-api-key') as HTMLInputElement | null;
  const keyHint = document.getElementById('settings-llm-key-hint');
  if (apiKeyInput) {
    apiKeyInput.value = engineKeySavedByCategory[categoryId] ? SAVED_KEY_PLACEHOLDER : '';
  }
  if (keyHint) keyHint.textContent = credentialHintForCategory(categoryId);
}

/** Focus on the six dots: empty the field so the user types a replacement. */
export function clearSavedCredentialPlaceholder(input: HTMLInputElement) {
  if (input.value === SAVED_KEY_PLACEHOLDER) input.value = '';
}

/** Blur with nothing typed: bring the six dots back when this category has a saved key. */
export function restoreSavedCredentialPlaceholder(input: HTMLInputElement) {
  if (input.value === '' && engineKeySavedByCategory[store.activeEngineCategory]) {
    input.value = SAVED_KEY_PLACEHOLDER;
  }
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
  const engineSelect = document.getElementById('settings-llm-engine') as HTMLSelectElement | null;
  const modelInput = document.getElementById('settings-llm-model') as HTMLInputElement | null;

  store.activeEngineCategory = categoryId;
  publishEngineCategory(categoryId);
  for (const category of listEngineCategories()) {
    const saved = llm[category.id] ?? {};
    const categoryPreset = getEnginePreset(category.id);
    engineModelByCategory[category.id] =
      (typeof saved.model === 'string' ? saved.model : '') || categoryPreset?.fields?.model || '';
    engineBaseUrlByCategory[category.id] =
      (typeof saved.base_url === 'string' ? saved.base_url.trim() : '')
      || categoryPreset?.fields?.base_url
      || '';
    engineKeySavedByCategory[category.id] = Boolean(saved.key_saved);
  }

  if (engineSelect) {
    engineSelect.value = categoryId;
    syncSettingsListSelect(engineSelect);
  }
  fillPresetFields(categoryId, {
    baseUrl: engineBaseUrlByCategory[categoryId],
    forceBaseUrl: true,
  });
  if (modelInput) {
    modelInput.value = engineModelByCategory[categoryId] ?? '';
    modelInput.readOnly = false;
    modelInput.disabled = false;
  }
  paintCredentialForCategory(categoryId);
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
  const preset = resolveEnginePreset(id);

  if (modelInput) {
    engineModelByCategory[prev] = modelInput.value;
  }
  if (baseUrlInput) {
    engineBaseUrlByCategory[prev] = baseUrlInput.value;
  }
  store.activeEngineCategory = id;
  publishEngineCategory(id);

  if (engineSelect) {
    engineSelect.value = id;
    syncSettingsListSelect(engineSelect);
  }
  fillPresetFields(id, {
    baseUrl: engineBaseUrlByCategory[id] ?? '',
    forceBaseUrl: true,
  });
  if (clearCredential) {
    paintCredentialForCategory(id);
  } else {
    const keyHint = document.getElementById('settings-llm-key-hint');
    if (keyHint) keyHint.textContent = credentialHintForCategory(id);
  }

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
  const typedKey = (document.getElementById('settings-llm-api-key') as HTMLInputElement | null)?.value.trim() ?? '';
  const apiKey = typedKey === SAVED_KEY_PLACEHOLDER ? '' : typedKey;

  const payload: Record<string, unknown> = {
    assistant_engine: categoryId,
    llm: { model, base_url: baseUrl },
  };
  if (apiKey) {
    payload.llm_api_key = { engine: categoryId, value: apiKey };
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
