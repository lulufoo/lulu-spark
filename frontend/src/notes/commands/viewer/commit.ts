import * as api from '../../../host/api.ts';
import { loadDiffStatus, notifyState, state } from '../../state/host.ts';
import { commitOpenStore } from '../../state/dialog-open.ts';
import {
  COMMIT_GROUPS,
  commitViewStore,
  emptyCommitView,
  patchCommit,
  type CommitGroupKey,
} from '../../state/commit.ts';

export { commitOpenStore };
export { commitViewStore } from '../../state/commit.ts';

type DiffStatus = {
  total?: number;
  ahead?: number;
  error?: string;
  new?: string[];
  modified?: string[];
  deleted?: string[];
  renamed?: string[];
  conflicted?: string[];
};

let revertAllTimer: ReturnType<typeof setTimeout> | null = null;

function clearRevertAllTimer() {
  if (revertAllTimer == null) return;
  clearTimeout(revertAllTimer);
  revertAllTimer = null;
}

function dualWriteCommitOpen(open: boolean) {
  const dialog = document.getElementById('md-commit-dialog');
  if (!dialog) return;
  if (open) dialog.classList.add('open');
  else dialog.classList.remove('open');
}

export function setCommitMessage(message: string) {
  patchCommit({ message });
}

export function showPendingBadge() {
  state.viewer.pendingCommit = true;
  const btn = document.getElementById('btn-panel-commit');
  if (btn) btn.style.display = '';
  notifyState();
}

export function hidePendingBadge() {
  state.viewer.pendingCommit = false;
  const btn = document.getElementById('btn-panel-commit');
  if (btn) btn.style.display = 'none';
  notifyState();
}

export function closeCommitDialog() {
  clearRevertAllTimer();
  commitOpenStore.set(false);
  commitViewStore.set(emptyCommitView());
  dualWriteCommitOpen(false);
}

export async function openCommitDialog() {
  clearRevertAllTimer();
  commitViewStore.set({
    ...emptyCommitView(),
    loading: true,
  });
  commitOpenStore.set(true);
  dualWriteCommitOpen(true);

  try {
    const data = (await api.fetchDiffStatus()) as DiffStatus | null;
    if (!data) throw new Error('Could not get status');
    if (data.error) throw new Error(data.error);

    if (!data.total && !data.ahead) {
      patchCommit({
        loading: false,
        error: '',
        groups: [],
        empty: true,
        canCommit: false,
      });
      return;
    }

    const groups = COMMIT_GROUPS.flatMap(({ key, label }) => {
      const files = data[key];
      if (!files?.length) return [];
      return [{ key, label, files }];
    });
    patchCommit({
      loading: false,
      error: '',
      groups,
      empty: false,
      canCommit: true,
    });
  } catch (err) {
    patchCommit({
      loading: false,
      error: (err as Error).message,
      groups: [],
      empty: false,
      canCommit: true,
    });
  }
}

export async function doMdCommit() {
  const view = commitViewStore.getSnapshot();
  const msg = view.message.trim() || 'update: edit via viewer';
  patchCommit({ committing: true, canCommit: false, result: 'Committing…', resultKind: 'busy' });
  try {
    const data = (await api.commitFiles(msg)) as { error?: string };
    if (data.error) throw new Error(data.error);
    patchCommit({ result: '✓ Pushed!', resultKind: 'ok' });
    await loadDiffStatus();
    notifyState();
    hidePendingBadge();
    setTimeout(() => closeCommitDialog(), 1500);
  } catch (e) {
    patchCommit({
      result: `✗ ${(e as Error).message}`,
      resultKind: 'err',
      committing: false,
      canCommit: true,
    });
  }
}

export async function doMdRevertAll() {
  const view = commitViewStore.getSnapshot();
  if (!view.revertAllConfirm) {
    patchCommit({ revertAllConfirm: true });
    revertAllTimer = setTimeout(() => {
      revertAllTimer = null;
      patchCommit({ revertAllConfirm: false });
    }, 3000);
    return;
  }
  clearRevertAllTimer();
  patchCommit({ revertAllConfirm: false, committing: true });
  try {
    const data = (await api.revertFile('', '')) as { error?: string };
    if (data.error) throw new Error(data.error);
    await loadDiffStatus();
    notifyState();
    hidePendingBadge();
    closeCommitDialog();
  } catch (e) {
    patchCommit({
      result: `Revert failed: ${(e as Error).message}`,
      resultKind: 'err',
      committing: false,
    });
  }
}

export async function doMdRevertFile(path: string, type: CommitGroupKey | string) {
  patchCommit({ revertingPath: path });
  try {
    const data = (await api.revertFile(path, type)) as { error?: string };
    if (data.error) throw new Error(data.error);
    await openCommitDialog();
  } catch (e) {
    patchCommit({
      revertingPath: null,
      result: `Revert failed: ${(e as Error).message}`,
      resultKind: 'err',
    });
  }
}
