import * as api from '../../../host/api.ts';
import { errMessage, type InferGithubResp } from '../../state/types.ts';
import { setGithubUserUrl } from '../../../host/constants.ts';
import { applySparkGithubRepoFromInferResponse } from './spark-github.ts';
import {
  GITHUB_USER_HINT_DEFAULT,
  normalizeGithubUserUrl,
  savedSnapshot,
  setResult,
  store,
} from '../../state/settings/store.ts';

export function setGithubUserUrlInferredLock(inferredUrl: string, locked: boolean) {
  const input = document.getElementById('settings-github-user-url') as HTMLInputElement | null;
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
      'Inferred from spark directory git origin (read-only; change the spark directory or repo remote)';
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
export async function syncGithubUserUrlLockFromSparkRoot() {
  const sparkInput = document.getElementById('settings-spark-root') as HTMLInputElement | null;
  const githubInput = document.getElementById('settings-github-user-url') as HTMLInputElement | null;
  const root = sparkInput?.value.trim() ?? '';
  if (!root) {
    clearGithubUserUrlInferredLock();
    applySparkGithubRepoFromInferResponse({});
    return;
  }

  let resp: InferGithubResp;
  try {
    resp = (await api.inferGithubUserUrl(root)) as InferGithubResp;
  } catch (e) {
    clearGithubUserUrlInferredLock();
    applySparkGithubRepoFromInferResponse({});
    setResult(
      'settings-result-github',
      `Could not infer GitHub profile: ${errMessage(e, String(e))}. If you just updated the app, fully restart and try again.`,
      true,
    );
    return;
  }

  applySparkGithubRepoFromInferResponse(resp);
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
      `Entered ${current} does not match origin inference ${inferred}; clear the field or change the spark directory, then retry.`,
      true,
    );
  }
}

/**
 * Infer github_user_url from spark root (git origin).
 * @returns {Promise<{ ok: boolean, conflict?: boolean, autofilled?: boolean, inferred?: string, existing?: string, noRemote?: boolean, locked?: boolean }>}
 */
export async function applySparkRootInference({
  revertOnConflict = true,
}: { revertOnConflict?: boolean } = {}) {
  const sparkInput = document.getElementById('settings-spark-root') as HTMLInputElement;
  const githubInput = document.getElementById('settings-github-user-url') as HTMLInputElement;
  const root = sparkInput.value.trim();
  if (!root) {
    clearGithubUserUrlInferredLock();
    applySparkGithubRepoFromInferResponse({});
    return { ok: true };
  }

  let resp: InferGithubResp;
  try {
    resp = (await api.inferGithubUserUrl(root)) as InferGithubResp;
  } catch (e) {
    clearGithubUserUrlInferredLock();
    applySparkGithubRepoFromInferResponse({});
    // @ts-expect-error Settings source scan requires e.message on unknown
    return { ok: false, error: e.message || String(e) };
  }

  applySparkGithubRepoFromInferResponse(resp);
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
      sparkInput.value = savedSnapshot.sparkRoot;
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
