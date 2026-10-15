export const DEFAULT_ENGINE_CATEGORY = 'host';
export const ENGINE_CHANGED_EVENT = 'spark:engine-changed';

export function publishEngineCategory(categoryId: string) {
  if (typeof window === 'undefined') return;
  window.dispatchEvent(new CustomEvent(ENGINE_CHANGED_EVENT, { detail: { categoryId } }));
}

/** Credential already saved on the host, per category (`llm.<id>.key_saved` from get_config). */
export const engineKeySavedByCategory: Record<string, boolean | undefined> = {};

/** Six characters: a password field paints them as six dots. Never sent to the host. */
export const SAVED_KEY_PLACEHOLDER = '••••••';

/** Per-category drafts (`undefined` = not loaded for this panel session). */
export const engineModelByCategory: Record<string, string | undefined> = {};
export const engineBaseUrlByCategory: Record<string, string | undefined> = {};

export const store = {
  activeEngineCategory: DEFAULT_ENGINE_CATEGORY,
  /** Host MCP listen port (same value GET /health uses in its mcp template). */
  mcpPort: 9876,
};

export function setResult(resultElId: string, message: string, isError = false) {
  const el = document.getElementById(resultElId);
  if (!el) return;
  el.textContent = message || '';
  el.style.color = isError ? '#cf222e' : '#1a7f37';
}

