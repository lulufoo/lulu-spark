import * as api from '../../../host/api.ts';
import {
  SPARK_CONNECT_NEEDS_ACCOUNT,
  sparkConnectionStore,
  normalizeGithubUserUrl,
  savedSnapshot,
  setResult,
  store,
} from '../../state/settings/store.ts';

export function normalizeSparkGithubRepoUrl(raw: string | null | undefined): string {
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

export function sparkGithubRepoFullName(url: string | null | undefined): string {
  const m = String(url || '').match(/^https:\/\/github\.com\/([^/]+\/[^/]+)/i);
  return m ? m[1] : String(url || '');
}

export function renderSparkConnection(repoUrl: string | null | undefined, { locked = false } = {}) {
  const addWrap = document.getElementById('spark-connect-add');
  const url = (repoUrl || '').trim();
  if (addWrap) addWrap.hidden = Boolean(url);
  sparkConnectionStore.set({ url, locked });
}

export function applySparkGithubRepoFromInferResponse(resp: { spark_github_repo_url?: string } | null | undefined) {
  const inferred = normalizeSparkGithubRepoUrl(resp?.spark_github_repo_url || '');
  if (inferred && !isGithubAccountConfigured()) {
    store.sparkGithubRepoInferredFromOrigin = '';
    renderSparkConnection(savedSnapshot.sparkGithubRepoUrl, { locked: false });
    setResult('spark-connect-error', SPARK_CONNECT_NEEDS_ACCOUNT, true);
    syncSparkConnectionAccess();
    return;
  }
  store.sparkGithubRepoInferredFromOrigin = inferred;
  if (inferred) {
    renderSparkConnection(inferred, { locked: true });
    setResult(
      'spark-connect-error',
      'Inferred from spark directory git origin (read-only).',
    );
    return;
  }
  renderSparkConnection(savedSnapshot.sparkGithubRepoUrl, { locked: false });
  setResult('spark-connect-error', '');
}

export async function saveSparkGithubRepoUrl(repoUrl: string) {
  const resp = await api.setConfig({ spark_github_repo_url: repoUrl });
  if (resp?.error) throw new Error(resp.error);
  savedSnapshot.sparkGithubRepoUrl = repoUrl;
  renderSparkConnection(repoUrl);
}

export function isGithubAccountConfigured() {
  if (savedSnapshot.hasGithubToken) return true;
  const n = normalizeGithubUserUrl(savedSnapshot.githubUserUrl);
  return /^https:\/\/github\.com\/[^/]+$/.test(n);
}

export function syncSparkConnectionAccess() {
  const allowed = isGithubAccountConfigured();
  const input = document.getElementById('spark-connect-url') as HTMLInputElement | null;
  const addBtn = document.getElementById('btn-spark-connect-add') as HTMLButtonElement | null;
  if (input) {
    input.disabled = !allowed;
    input.placeholder = allowed ? 'owner/repo or GitHub URL' : 'Set a Sync token first';
  }
  if (addBtn) addBtn.disabled = !allowed;
  if (!allowed && !store.sparkGithubRepoInferredFromOrigin) {
    setResult('spark-connect-error', SPARK_CONNECT_NEEDS_ACCOUNT, true);
  } else if (allowed) {
    const el = document.getElementById('spark-connect-error');
    if (el?.textContent === SPARK_CONNECT_NEEDS_ACCOUNT) {
      setResult('spark-connect-error', '');
    }
  }
}

export async function addSparkGithubRepo() {
  const input = document.getElementById('spark-connect-url') as HTMLInputElement | null;
  const btn = document.getElementById('btn-spark-connect-add') as HTMLButtonElement | null;
  const url = normalizeSparkGithubRepoUrl(input?.value || '');
  if (!isGithubAccountConfigured()) {
    setResult('spark-connect-error', SPARK_CONNECT_NEEDS_ACCOUNT, true);
    return;
  }
  if (store.sparkGithubRepoInferredFromOrigin) {
    setResult('spark-connect-error', 'Inferred from git origin (read-only).', true);
    return;
  }
  if (!url) {
    setResult('spark-connect-error', 'Enter a GitHub repository URL.', true);
    return;
  }
  if (savedSnapshot.sparkGithubRepoUrl) {
    setResult('spark-connect-error', 'Delete the current repository before adding another.', true);
    return;
  }
  if (btn) btn.disabled = true;
  setResult('spark-connect-error', '');
  try {
    await saveSparkGithubRepoUrl(url);
    if (input) input.value = '';
    setResult('spark-connect-error', 'Added.');
  } catch (e) {
    setResult('spark-connect-error', e instanceof Error ? e.message : String(e), true);
  } finally {
    if (btn) btn.disabled = false;
  }
}

export async function deleteSparkGithubRepo() {
  if (store.sparkGithubRepoInferredFromOrigin) {
    setResult('spark-connect-error', 'Inferred from git origin (read-only).', true);
    return;
  }
  const btn = document.getElementById('btn-spark-connect-delete') as HTMLButtonElement | null;
  if (btn) btn.disabled = true;
  setResult('spark-connect-error', '');
  try {
    await saveSparkGithubRepoUrl('');
    setResult('spark-connect-error', 'Removed. You can add a repository again.');
  } catch (e) {
    setResult('spark-connect-error', e instanceof Error ? e.message : String(e), true);
    renderSparkConnection(savedSnapshot.sparkGithubRepoUrl);
  }
}
