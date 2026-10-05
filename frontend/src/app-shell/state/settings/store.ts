export const DEFAULT_ENGINE_CATEGORY = 'host';

export const engineKeyHints = {
  has_host_key: false,
};

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

