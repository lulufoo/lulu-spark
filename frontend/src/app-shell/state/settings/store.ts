import { createModuleStore } from '../../../shared/module-store.ts';

export const GITHUB_USER_HINT_DEFAULT =
  'Inferred from the Workbench directory origin when possible; used for Viewer remote links.';
export const WORKBENCH_CONNECT_NEEDS_ACCOUNT =
  'Set a Sync token first to bind a Workbench repository.';
export const DEFAULT_ENGINE_CATEGORY = 'host';

export const engineKeyHints = {
  has_host_key: false,
};

/** Host model draft (`undefined` = not loaded for this panel session). */
export const engineModelByCategory: { host: string | undefined } = {
  host: undefined,
};

export const savedSnapshot = {
  workbenchRoot: '',
  githubUserUrl: '',
  workbenchGithubRepoUrl: '',
  hasGithubToken: false,
};

export const store = {
  activeEngineCategory: DEFAULT_ENGINE_CATEGORY as 'host',
  /** Host MCP listen port (same value GET /health uses in its mcp template). */
  mcpPort: 9876,
  githubUserUrlInferredFromOrigin: '',
  workbenchGithubRepoInferredFromOrigin: '',
};

export function setResult(resultElId: string, message: string, isError = false) {
  const el = document.getElementById(resultElId);
  if (!el) return;
  el.textContent = message || '';
  el.style.color = isError ? '#cf222e' : '#1a7f37';
}

export function normalizeGithubUserUrl(url: string | null | undefined): string {
  return (url || '').trim().replace(/\/$/, '').toLowerCase();
}

export function isGithubUserUrlInferredLocked(): boolean {
  return Boolean(store.githubUserUrlInferredFromOrigin);
}

export const workbenchConnectionStore = createModuleStore({
  url: '',
  locked: false,
});
