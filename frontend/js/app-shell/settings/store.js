export const GITHUB_USER_HINT_DEFAULT =
  'Inferred from the Notes directory origin when possible; used for Viewer remote links.';
export const NOTES_CONNECT_NEEDS_ACCOUNT =
  'Set a Sync token first to bind a Notes repository.';
export const DEFAULT_ENGINE_CATEGORY = 'host';

/** @type {{ has_host_key: boolean }} */
export const engineKeyHints = {
  has_host_key: false,
};

/** Host model draft (`undefined` = not loaded for this panel session). */
/** @type {{ host: string|undefined }} */
export const engineModelByCategory = {
  host: undefined,
};

export const savedSnapshot = {
  workbenchKnowledgeRoot: '',
  githubUserUrl: '',
  workbenchGithubRepoUrl: '',
  hasGithubToken: false,
};

export const store = {
  /** @type {'host'} */
  activeEngineCategory: DEFAULT_ENGINE_CATEGORY,
  /** Host MCP listen port (same value GET /health uses in its mcp template). */
  mcpPort: 9876,
  githubUserUrlInferredFromOrigin: '',
  notesGithubRepoInferredFromOrigin: '',
};

export function setResult(resultElId, message, isError = false) {
  const el = document.getElementById(resultElId);
  if (!el) return;
  el.textContent = message || '';
  el.style.color = isError ? '#cf222e' : '#1a7f37';
}

export function normalizeGithubUserUrl(url) {
  return (url || '').trim().replace(/\/$/, '').toLowerCase();
}

export function isGithubUserUrlInferredLocked() {
  return Boolean(store.githubUserUrlInferredFromOrigin);
}
