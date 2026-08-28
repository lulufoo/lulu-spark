// @ts-nocheck — ported from JS; settings DOM wiring stays unchecked like checkJs:false.
import * as api from '../../../host/api.ts';
import { setGithubUserUrl } from '../../../host/constants.ts';
import { applyNotesGithubRepoFromInferResponse } from './notes-github.ts';
import {
  GITHUB_USER_HINT_DEFAULT,
  normalizeGithubUserUrl,
  savedSnapshot,
  setResult,
  store,
} from '../../state/settings/store.ts';

export function setGithubUserUrlInferredLock(inferredUrl, locked) {
  const input = document.getElementById('settings-github-user-url');
  const hint = document.getElementById('settings-github-user-hint');
  if (!input || !hint) return;

  if (locked && inferredUrl) {
    store.githubUserUrlInferredFromOrigin = inferredUrl;
    input.value = inferredUrl;
    input.readOnly = true;
    input.disabled = true;
    input.classList.add('settings-input-readonly');
    setGithubUserUrl(inferredUrl);
    hint.textContent =
      'Inferred from workbench directory git origin (read-only; change the workbench directory or repo remote)';
    hint.style.color = '#1a7f37';
  } else {
    store.githubUserUrlInferredFromOrigin = '';
    input.readOnly = false;
    input.disabled = false;
    input.classList.remove('settings-input-readonly');
    hint.textContent = GITHUB_USER_HINT_DEFAULT;
    hint.style.color = '';
  }
}

export function clearGithubUserUrlInferredLock() {
  setGithubUserUrlInferredLock('', false);
}
export async function syncGithubUserUrlLockFromWorkbenchRoot() {
  const archiveInput = document.getElementById('settings-archive-root');
  const githubInput = document.getElementById('settings-github-user-url');
  const root = archiveInput?.value.trim() ?? '';
  if (!root) {
    clearGithubUserUrlInferredLock();
    applyNotesGithubRepoFromInferResponse({});
    return;
  }

  let resp;
  try {
    resp = await api.inferGithubUserUrl(root);
  } catch (e) {
    clearGithubUserUrlInferredLock();
    applyNotesGithubRepoFromInferResponse({});
    setResult(
      'settings-result-github',
      `Could not infer GitHub profile: ${e.message || String(e)}. If you just updated the app, fully restart and try again.`,
      true,
    );
    return;
  }

  applyNotesGithubRepoFromInferResponse(resp);
  const inferred = (resp?.github_user_url || '').trim();
  if (!inferred) {
    clearGithubUserUrlInferredLock();
    setResult(
      'settings-result-github',
      'No git origin detected; could not infer GitHub profile — enter it manually.',
      false,
    );
    return;
  }

  const current = githubInput?.value.trim() ?? '';
  if (!current || normalizeGithubUserUrl(current) === normalizeGithubUserUrl(inferred)) {
    setGithubUserUrlInferredLock(inferred, true);
    setResult('settings-result-github', '');
  } else {
    clearGithubUserUrlInferredLock();
    setResult(
      'settings-result-github',
      `Entered ${current} does not match origin inference ${inferred}; clear the field or change the workbench directory, then retry.`,
      true,
    );
  }
}

/**
 * Infer github_user_url from workbench root (git origin).
 * @returns {Promise<{ ok: boolean, conflict?: boolean, autofilled?: boolean, inferred?: string, existing?: string, noRemote?: boolean, locked?: boolean }>}
 */
export async function applyWorkbenchRootInference({ revertOnConflict = true } = {}) {
  const archiveInput = document.getElementById('settings-archive-root');
  const githubInput = document.getElementById('settings-github-user-url');
  const root = archiveInput.value.trim();
  if (!root) {
    clearGithubUserUrlInferredLock();
    applyNotesGithubRepoFromInferResponse({});
    return { ok: true };
  }

  let resp;
  try {
    resp = await api.inferGithubUserUrl(root);
  } catch (e) {
    clearGithubUserUrlInferredLock();
    applyNotesGithubRepoFromInferResponse({});
    return { ok: false, error: e.message || String(e) };
  }

  applyNotesGithubRepoFromInferResponse(resp);
  const inferred = (resp?.github_user_url || '').trim();
  if (!inferred) {
    clearGithubUserUrlInferredLock();
    return { ok: true, noRemote: true };
  }

  const existing = githubInput.value.trim();
  const existingNorm = normalizeGithubUserUrl(existing);
  const inferredNorm = normalizeGithubUserUrl(inferred);

  if (existing && existingNorm !== inferredNorm) {
    if (revertOnConflict) {
      archiveInput.value = savedSnapshot.workbenchKnowledgeRoot;
    }
    clearGithubUserUrlInferredLock();
    return { ok: false, conflict: true, existing, inferred };
  }

  setGithubUserUrlInferredLock(inferred, true);
  return {
    ok: true,
    autofilled: !existing,
    inferred,
    locked: true,
  };
}
