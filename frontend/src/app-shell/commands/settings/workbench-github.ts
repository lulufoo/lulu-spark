import * as api from '../../../host/api.ts';
import {
  WORKBENCH_CONNECT_NEEDS_ACCOUNT,
  workbenchConnectionStore,
  normalizeGithubUserUrl,
  savedSnapshot,
  setResult,
  store,
} from '../../state/settings/store.ts';

export function normalizeWorkbenchGithubRepoUrl(raw: string | null | undefined): string {
  const s = (raw || '').trim().replace(/\/+$/, '').replace(/\.git$/i, '');
  if (!s) return '';
  let ownerRepo = '';
  const https = s.match(/^https?:\/\/github\.com\/([^/]+)\/([^/#?]+)/i);
  const ssh = s.match(/^git@github\.com:([^/]+)\/([^/#?]+)/i);
  if (https) ownerRepo = `${https[1]}/${https[2]}`;
  else if (ssh) ownerRepo = `${ssh[1]}/${ssh[2]}`;
  else if (/^[^/\s]+\/[^/\s]+$/.test(s)) ownerRepo = s;
  else return '';
  ownerRepo = ownerRepo.replace(/\.git$/i, '');
  return `https://github.com/${ownerRepo}`;
}

export function workbenchGithubRepoFullName(url: string | null | undefined): string {
  const m = String(url || '').match(/^https:\/\/github\.com\/([^/]+\/[^/]+)/i);
  return m ? m[1] : String(url || '');
}

export function renderWorkbenchConnection(repoUrl: string | null | undefined, { locked = false } = {}) {
  const addWrap = document.getElementById('workbench-connect-add');
  const url = (repoUrl || '').trim();
  if (addWrap) addWrap.hidden = Boolean(url);
  workbenchConnectionStore.set({ url, locked });
}

export function applyWorkbenchGithubRepoFromInferResponse(resp: { spark_github_repo_url?: string } | null | undefined) {
  const inferred = normalizeWorkbenchGithubRepoUrl(resp?.spark_github_repo_url || '');
  if (inferred && !isGithubAccountConfigured()) {
    store.workbenchGithubRepoInferredFromOrigin = '';
    renderWorkbenchConnection(savedSnapshot.workbenchGithubRepoUrl, { locked: false });
    setResult('workbench-connect-error', WORKBENCH_CONNECT_NEEDS_ACCOUNT, true);
    syncWorkbenchConnectionAccess();
    return;
  }
  store.workbenchGithubRepoInferredFromOrigin = inferred;
  if (inferred) {
    renderWorkbenchConnection(inferred, { locked: true });
    setResult(
      'workbench-connect-error',
      'Inferred from workbench directory git origin (read-only).',
    );
    return;
  }
  renderWorkbenchConnection(savedSnapshot.workbenchGithubRepoUrl, { locked: false });
  setResult('workbench-connect-error', '');
}

export async function saveWorkbenchGithubRepoUrl(repoUrl: string) {
  const resp = await api.setConfig({ spark_github_repo_url: repoUrl });
  if (resp?.error) throw new Error(resp.error);
  savedSnapshot.workbenchGithubRepoUrl = repoUrl;
  renderWorkbenchConnection(repoUrl);
}

export function isGithubAccountConfigured() {
  if (savedSnapshot.hasGithubToken) return true;
  const n = normalizeGithubUserUrl(savedSnapshot.githubUserUrl);
  return /^https:\/\/github\.com\/[^/]+$/.test(n);
}

export function syncWorkbenchConnectionAccess() {
  const allowed = isGithubAccountConfigured();
  const input = document.getElementById('workbench-connect-url') as HTMLInputElement | null;
  const addBtn = document.getElementById('btn-workbench-connect-add') as HTMLButtonElement | null;
  if (input) {
    input.disabled = !allowed;
    input.placeholder = allowed ? 'owner/repo or GitHub URL' : 'Set a Sync token first';
  }
  if (addBtn) addBtn.disabled = !allowed;
  if (!allowed && !store.workbenchGithubRepoInferredFromOrigin) {
    setResult('workbench-connect-error', WORKBENCH_CONNECT_NEEDS_ACCOUNT, true);
  } else if (allowed) {
    const el = document.getElementById('workbench-connect-error');
    if (el?.textContent === WORKBENCH_CONNECT_NEEDS_ACCOUNT) {
      setResult('workbench-connect-error', '');
    }
  }
}

export async function addWorkbenchGithubRepo() {
  const input = document.getElementById('workbench-connect-url') as HTMLInputElement | null;
  const btn = document.getElementById('btn-workbench-connect-add') as HTMLButtonElement | null;
  const url = normalizeWorkbenchGithubRepoUrl(input?.value || '');
  if (!isGithubAccountConfigured()) {
    setResult('workbench-connect-error', WORKBENCH_CONNECT_NEEDS_ACCOUNT, true);
    return;
  }
  if (store.workbenchGithubRepoInferredFromOrigin) {
    setResult('workbench-connect-error', 'Inferred from git origin (read-only).', true);
    return;
  }
  if (!url) {
    setResult('workbench-connect-error', 'Enter a GitHub repository URL.', true);
    return;
  }
  if (savedSnapshot.workbenchGithubRepoUrl) {
    setResult('workbench-connect-error', 'Delete the current repository before adding another.', true);
    return;
  }
  if (btn) btn.disabled = true;
  setResult('workbench-connect-error', '');
  try {
    await saveWorkbenchGithubRepoUrl(url);
    if (input) input.value = '';
    setResult('workbench-connect-error', 'Added.');
  } catch (e) {
    setResult('workbench-connect-error', e instanceof Error ? e.message : String(e), true);
  } finally {
    if (btn) btn.disabled = false;
  }
}

export async function deleteWorkbenchGithubRepo() {
  if (store.workbenchGithubRepoInferredFromOrigin) {
    setResult('workbench-connect-error', 'Inferred from git origin (read-only).', true);
    return;
  }
  const btn = document.getElementById('btn-workbench-connect-delete') as HTMLButtonElement | null;
  if (btn) btn.disabled = true;
  setResult('workbench-connect-error', '');
  try {
    await saveWorkbenchGithubRepoUrl('');
    setResult('workbench-connect-error', 'Removed. You can add a repository again.');
  } catch (e) {
    setResult('workbench-connect-error', e instanceof Error ? e.message : String(e), true);
    renderWorkbenchConnection(savedSnapshot.workbenchGithubRepoUrl);
  }
}
