// @ts-nocheck — DOM wiring stays unchecked like checkJs:false.
import * as api from '../../host/api.ts';
import { openKbDiffDialog } from '../../corpus/ui/diff-dialog.tsx';

let corpusRepo = '';
/** @type {(() => void | Promise<void>) | null} */
let onCorpusRefresh = null;

export function setHeaderSyncCorpusContext(repo, onRefresh) {
  corpusRepo = repo || '';
  onCorpusRefresh = typeof onRefresh === 'function' ? onRefresh : null;
}

export function clearHeaderSyncCorpusContext() {
  corpusRepo = '';
  onCorpusRefresh = null;
}

async function pullCorpusRepo(repo) {
  await api.reindexKbRepo(repo);
  for (let i = 0; i < 120; i++) {
    await new Promise((r) => setTimeout(r, 2000));
    const status = await api.getReindexStatus();
    if (status?.status !== 'running') break;
  }
  if (onCorpusRefresh) await onCorpusRefresh();
}

/**
 * @param {{ pullProject: () => Promise<void>, loadIndex: () => Promise<void>, openWorkbenchCommit?: () => void | Promise<void> }} deps
 */
export function initHeaderSync({ pullProject, loadIndex, openWorkbenchCommit }) {
  const openWorkbench = openWorkbenchCommit ?? (() =>
    import('./commit-dialog.ts').then(({ openCommitChangesDialog }) => {
      void openCommitChangesDialog();
    }));

  document.getElementById('btn-push-index')?.addEventListener('click', () => {
    if (corpusRepo) {
      void openKbDiffDialog(corpusRepo);
      return;
    }
    void openWorkbench();
  });

  document.getElementById('btn-pull')?.addEventListener('click', () => {
    const btn = document.getElementById('btn-pull');
    if (!btn) return;
    if (corpusRepo) {
      btn.disabled = true;
      btn.textContent = 'Updating…';
      void pullCorpusRepo(corpusRepo)
        .catch((err) => {
          alert(`Update failed: ${err?.message || 'Unknown error'}`);
        })
        .finally(() => {
          btn.disabled = false;
          btn.textContent = '↓ Update project';
        });
      return;
    }
    void pullProject();
  });

  document.getElementById('btn-local-refresh')?.addEventListener('click', () => {
    const btn = document.getElementById('btn-local-refresh');
    if (!btn) return;
    btn.disabled = true;
    btn.textContent = '⟳ Refreshing…';
    const finish = () => {
      btn.disabled = false;
      btn.textContent = '⟳ Refresh local';
    };
    if (corpusRepo && onCorpusRefresh) {
      void Promise.resolve(onCorpusRefresh()).finally(finish);
      return;
    }
    loadIndex().finally(finish);
  });
}
