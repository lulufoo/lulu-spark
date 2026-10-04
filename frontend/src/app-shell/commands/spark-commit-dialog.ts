import * as api from '../../host/api.ts';
import { showToast } from '../../toast.tsx';
import {
  SPARK_COMMIT_GROUPS,
  emptySparkCommitView,
  patchSparkCommit,
  sparkCommitOpenStore,
  sparkCommitViewStore,
} from '../state/spark-commit.ts';

export { sparkCommitOpenStore };
export { sparkCommitViewStore } from '../state/spark-commit.ts';

const HEADER_LABEL_IDLE = '↑ Commit changes';
const HEADER_LABEL_CHECKING = 'Checking…';
const CLOSE_DELAY_MS = 500;

type DiffStatus = {
  total?: number;
  ahead?: number;
  error?: string;
  new?: string[];
  modified?: string[];
  renamed?: string[];
  deleted?: string[];
  conflicted?: string[];
};

let commitCloseTimer: ReturnType<typeof setTimeout> | null = null;

function clearCommitCloseTimer() {
  if (commitCloseTimer != null) {
    clearTimeout(commitCloseTimer);
    commitCloseTimer = null;
  }
}

function dualWriteOpen(open: boolean) {
  const dialog = document.getElementById('spark-commit-dialog');
  if (!dialog) return;
  if (open) dialog.classList.add('open');
  else dialog.classList.remove('open');
}

function setHeaderBusy(busy: boolean) {
  const headerBtn = document.getElementById('btn-push-index') as HTMLButtonElement | null;
  if (!headerBtn) return;
  headerBtn.disabled = busy;
  headerBtn.textContent = busy ? HEADER_LABEL_CHECKING : HEADER_LABEL_IDLE;
}

export function setSparkCommitMessage(message: string) {
  patchSparkCommit({ message });
}

export async function openSparkCommitDialog() {
  setHeaderBusy(true);
  sparkCommitViewStore.set({ ...emptySparkCommitView(), loading: true });
  sparkCommitOpenStore.set(true);
  dualWriteOpen(true);

  try {
    const data = (await api.fetchDiffStatus()) as DiffStatus | null;
    if (!data) throw new Error('Could not get status');
    if (data.error) throw new Error(data.error);

    if (!data.total && !data.ahead) {
      patchSparkCommit({
        loading: false,
        empty: true,
        groups: [],
        ahead: 0,
        canCommit: false,
        okLabel: 'Commit',
      });
    } else {
      const groups = SPARK_COMMIT_GROUPS.flatMap(({ key, label }) => {
        const files = data[key];
        if (!files?.length) return [];
        return [{ key, label, files }];
      });
      patchSparkCommit({
        loading: false,
        empty: false,
        error: '',
        groups,
        ahead: data.ahead || 0,
        canCommit: true,
        okLabel: data.total ? 'Commit' : 'Push',
      });
    }
  } catch (err) {
    patchSparkCommit({
      loading: false,
      error: (err as Error).message,
      canCommit: true,
    });
  } finally {
    setHeaderBusy(false);
  }
}

export function closeSparkCommitDialog() {
  sparkCommitOpenStore.set(false);
  clearCommitCloseTimer();
  sparkCommitViewStore.set(emptySparkCommitView());
  dualWriteOpen(false);
  const headerBtn = document.getElementById('btn-push-index') as HTMLButtonElement | null;
  if (headerBtn) {
    headerBtn.disabled = false;
    headerBtn.textContent = HEADER_LABEL_IDLE;
  }
}

export function doSparkCommit() {
  const leftoverMsg = (document.getElementById('spark-commit-msg') as HTMLInputElement | null)?.value;
  const msg = (leftoverMsg ?? sparkCommitViewStore.getSnapshot().message).trim();
  clearCommitCloseTimer();
  commitCloseTimer = setTimeout(() => {
    commitCloseTimer = null;
    closeSparkCommitDialog();

    void api
      .commitFiles(msg || 'chore: update via viewer')
      .then((data: { info?: string }) => {
        const successMsg = data?.info === 'nothing to commit' ? '✓ Pushed' : '✓ Committed and pushed';
        showToast(successMsg, 'success');
      })
      .catch((e: Error) => {
        showToast(`Commit failed: ${e.message}`, 'error');
      });
  }, CLOSE_DELAY_MS);
}
