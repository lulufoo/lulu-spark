import * as api from '../../../host/api.ts';
import {
  NOTES_CONNECT_NEEDS_ACCOUNT,
  notesConnectionStore,
  normalizeGithubUserUrl,
  savedSnapshot,
  setResult,
  store,
} from '../../state/settings/store.ts';

export function normalizeNotesGithubRepoUrl(raw: string | null | undefined): string {
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

export function notesGithubRepoFullName(url: string | null | undefined): string {
  const m = String(url || '').match(/^https:\/\/github\.com\/([^/]+\/[^/]+)/i);
  return m ? m[1] : String(url || '');
}

export function renderNotesConnection(repoUrl: string | null | undefined, { locked = false } = {}) {
  const addWrap = document.getElementById('notes-connect-add');
  const url = (repoUrl || '').trim();
  if (addWrap) addWrap.hidden = Boolean(url);
  notesConnectionStore.set({ url, locked });
}

export function applyNotesGithubRepoFromInferResponse(resp: { workbench_github_repo_url?: string } | null | undefined) {
  const inferred = normalizeNotesGithubRepoUrl(resp?.workbench_github_repo_url || '');
  if (inferred && !isGithubAccountConfigured()) {
    store.notesGithubRepoInferredFromOrigin = '';
    renderNotesConnection(savedSnapshot.workbenchGithubRepoUrl, { locked: false });
    setResult('notes-connect-error', NOTES_CONNECT_NEEDS_ACCOUNT, true);
    syncNotesConnectionAccess();
    return;
  }
  store.notesGithubRepoInferredFromOrigin = inferred;
  if (inferred) {
    renderNotesConnection(inferred, { locked: true });
    setResult(
      'notes-connect-error',
      'Inferred from workbench directory git origin (read-only).',
    );
    return;
  }
  renderNotesConnection(savedSnapshot.workbenchGithubRepoUrl, { locked: false });
  setResult('notes-connect-error', '');
}

export async function saveNotesGithubRepoUrl(repoUrl: string) {
  const resp = await api.setConfig({ workbench_github_repo_url: repoUrl });
  if (resp?.error) throw new Error(resp.error);
  savedSnapshot.workbenchGithubRepoUrl = repoUrl;
  renderNotesConnection(repoUrl);
}

export function isGithubAccountConfigured() {
  if (savedSnapshot.hasGithubToken) return true;
  const n = normalizeGithubUserUrl(savedSnapshot.githubUserUrl);
  return /^https:\/\/github\.com\/[^/]+$/.test(n);
}

export function syncNotesConnectionAccess() {
  const allowed = isGithubAccountConfigured();
  const input = document.getElementById('notes-connect-url') as HTMLInputElement | null;
  const addBtn = document.getElementById('btn-notes-connect-add') as HTMLButtonElement | null;
  if (input) {
    input.disabled = !allowed;
    input.placeholder = allowed ? 'owner/repo or GitHub URL' : 'Set a Sync token first';
  }
  if (addBtn) addBtn.disabled = !allowed;
  if (!allowed && !store.notesGithubRepoInferredFromOrigin) {
    setResult('notes-connect-error', NOTES_CONNECT_NEEDS_ACCOUNT, true);
  } else if (allowed) {
    const el = document.getElementById('notes-connect-error');
    if (el?.textContent === NOTES_CONNECT_NEEDS_ACCOUNT) {
      setResult('notes-connect-error', '');
    }
  }
}

export async function addNotesGithubRepo() {
  const input = document.getElementById('notes-connect-url') as HTMLInputElement | null;
  const btn = document.getElementById('btn-notes-connect-add') as HTMLButtonElement | null;
  const url = normalizeNotesGithubRepoUrl(input?.value || '');
  if (!isGithubAccountConfigured()) {
    setResult('notes-connect-error', NOTES_CONNECT_NEEDS_ACCOUNT, true);
    return;
  }
  if (store.notesGithubRepoInferredFromOrigin) {
    setResult('notes-connect-error', 'Inferred from git origin (read-only).', true);
    return;
  }
  if (!url) {
    setResult('notes-connect-error', 'Enter a GitHub repository URL.', true);
    return;
  }
  if (savedSnapshot.workbenchGithubRepoUrl) {
    setResult('notes-connect-error', 'Delete the current repository before adding another.', true);
    return;
  }
  if (btn) btn.disabled = true;
  setResult('notes-connect-error', '');
  try {
    await saveNotesGithubRepoUrl(url);
    if (input) input.value = '';
    setResult('notes-connect-error', 'Added.');
  } catch (e) {
    setResult('notes-connect-error', e instanceof Error ? e.message : String(e), true);
  } finally {
    if (btn) btn.disabled = false;
  }
}

export async function deleteNotesGithubRepo() {
  if (store.notesGithubRepoInferredFromOrigin) {
    setResult('notes-connect-error', 'Inferred from git origin (read-only).', true);
    return;
  }
  const btn = document.getElementById('btn-notes-connect-delete') as HTMLButtonElement | null;
  if (btn) btn.disabled = true;
  setResult('notes-connect-error', '');
  try {
    await saveNotesGithubRepoUrl('');
    setResult('notes-connect-error', 'Removed. You can add a repository again.');
  } catch (e) {
    setResult('notes-connect-error', e instanceof Error ? e.message : String(e), true);
    renderNotesConnection(savedSnapshot.workbenchGithubRepoUrl);
  }
}
