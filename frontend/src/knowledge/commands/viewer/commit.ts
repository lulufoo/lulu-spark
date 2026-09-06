import { state } from '../../state/host.ts';
import * as api from '../../../host/api.ts';
import { renderKbComments } from '../../ui/comments.tsx';
import { applyKbHighlights } from '../../ui/viewer/highlight.ts';
import { kbHidePendingBadge, kbPendingMsg } from '../../ui/viewer/chrome.tsx';
import {
  KB_COMMIT_GROUPS,
  emptyKbCommitView,
  kbCommitViewStore,
  patchKbCommit,
} from '../../state/commit.ts';
import { kbCommitOpenStore } from '../../state/dialog-open.ts';

export { kbCommitOpenStore };
export { kbCommitViewStore } from '../../state/commit.ts';

const REVERTABLE = new Set(['new', 'modified', 'deleted']);

type KbStatus = {
  total?: number;
  ahead?: number;
  error?: string;
  new?: string[];
  modified?: string[];
  renamed?: string[];
  deleted?: string[];
  conflicted?: string[];
};

type KbViewer = {
  kbRepo: string | null;
  kbPath: string | null;
  annotation: unknown;
};

function viewer(): KbViewer {
  return state.viewer as KbViewer;
}

function dualWriteOpen(open: boolean) {
  const dialog = document.getElementById('kb-commit-dialog');
  if (!dialog) return;
  if (open) dialog.classList.add('open');
  else dialog.classList.remove('open');
}

let revertAllTimer: ReturnType<typeof setTimeout> | null = null;

function clearRevertAllTimer() {
  if (revertAllTimer == null) return;
  clearTimeout(revertAllTimer);
  revertAllTimer = null;
}

export function setKbCommitMessage(message: string) {
  patchKbCommit({ message });
}

export async function refreshKbCommitFileList() {
  patchKbCommit({ loading: true, error: '', result: '', resultKind: '' });
  try {
    const data = (await api.fetchKbStatus(viewer().kbRepo)) as KbStatus;
    if (data.error) throw new Error(data.error);

    if (!data.total && !data.ahead) {
      patchKbCommit({
        loading: false,
        empty: true,
        groups: [],
        ahead: 0,
        canCommit: false,
        revertAllDisabled: true,
        okLabel: 'Commit',
      });
      return;
    }

    const groups = KB_COMMIT_GROUPS.flatMap(({ key, label }) => {
      const files = data[key];
      if (!files?.length) return [];
      return [{ key, label, files }];
    });
    patchKbCommit({
      loading: false,
      empty: false,
      error: '',
      groups,
      ahead: data.ahead || 0,
      canCommit: true,
      revertAllDisabled: false,
      okLabel: data.total ? 'Commit' : 'Push',
      revertingPath: null,
    });
  } catch (e) {
    patchKbCommit({
      loading: false,
      error: e instanceof Error ? e.message : String(e),
      canCommit: true,
    });
  }
}

export async function openKbCommitDialog() {
  clearRevertAllTimer();
  kbCommitViewStore.set({
    ...emptyKbCommitView(),
    message: kbPendingMsg() || '',
    loading: true,
  });
  kbCommitOpenStore.set(true);
  dualWriteOpen(true);
  await refreshKbCommitFileList();
}

export function closeKbCommitDialog() {
  clearRevertAllTimer();
  kbCommitOpenStore.set(false);
  kbCommitViewStore.set(emptyKbCommitView());
  dualWriteOpen(false);
}

export async function doKbCommit() {
  const view = kbCommitViewStore.getSnapshot();
  const msg = view.message.trim() || 'chore: update via viewer';
  patchKbCommit({ committing: true, canCommit: false, result: 'Committing…', resultKind: 'busy' });
  try {
    const data = (await api.commitKbFile(viewer().kbRepo, msg)) as { error?: string; stderr?: string };
    if (data.error) throw new Error(data.error + (data.stderr ? `\n${data.stderr}` : ''));
    patchKbCommit({ result: '✓ Committed and pushed', resultKind: 'ok' });
    kbHidePendingBadge();
    setTimeout(closeKbCommitDialog, 1500);
  } catch (e) {
    patchKbCommit({
      result: `✗ ${e instanceof Error ? e.message : String(e)}`,
      resultKind: 'err',
      committing: false,
      canCommit: true,
    });
  }
}

async function syncAnnotationIfNeeded(path: string) {
  const v = viewer();
  if (!v.kbPath || !path.startsWith('.knowledge_annotations')) return;
  try {
    const ann = await api.fetchKbAnnotation(v.kbRepo, v.kbPath);
    v.annotation = ann;
    renderKbComments(ann);
    void applyKbHighlights();
  } catch {
    /* best-effort UI sync; revert already succeeded */
  }
}

export async function kbRevertFile(path: string, type: string) {
  patchKbCommit({ revertingPath: path });
  try {
    const data = (await api.revertKbFile(viewer().kbRepo, path, type)) as { error?: string };
    if (data.error) throw new Error(data.error);
    await refreshKbCommitFileList();
    await syncAnnotationIfNeeded(path);
    const next = kbCommitViewStore.getSnapshot();
    if (!next.groups.length && !next.ahead) {
      kbHidePendingBadge();
      setTimeout(closeKbCommitDialog, 800);
    }
  } catch (e) {
    patchKbCommit({
      revertingPath: null,
      result: `Revert failed: ${e instanceof Error ? e.message : String(e)}`,
      resultKind: 'err',
    });
  }
}

export async function kbRevertAll() {
  const view = kbCommitViewStore.getSnapshot();
  if (!view.revertAllConfirm) {
    patchKbCommit({ revertAllConfirm: true });
    revertAllTimer = setTimeout(() => {
      revertAllTimer = null;
      patchKbCommit({ revertAllConfirm: false });
    }, 3000);
    return;
  }
  clearRevertAllTimer();
  patchKbCommit({ revertAllConfirm: false, committing: true, revertAllDisabled: true });
  try {
    const data = (await api.revertKbFile(viewer().kbRepo)) as { error?: string };
    if (data.error) throw new Error(data.error);
    kbHidePendingBadge();
    await refreshKbCommitFileList();
    const v = viewer();
    if (v.kbPath) {
      try {
        const ann = await api.fetchKbAnnotation(v.kbRepo, v.kbPath);
        v.annotation = ann;
        renderKbComments(ann);
        void applyKbHighlights();
      } catch {
        /* best-effort UI sync; revert already succeeded */
      }
    }
  } catch (e) {
    patchKbCommit({
      result: `✗ Revert failed: ${e instanceof Error ? e.message : String(e)}`,
      resultKind: 'err',
      committing: false,
      revertAllDisabled: false,
      revertAllConfirm: false,
    });
  }
}

export { REVERTABLE };
