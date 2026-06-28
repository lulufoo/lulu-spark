import * as api from './api.js';
import { openKbDiffDialog } from './components/modals/kb-diff-dialog.js';

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
    import('./components/modals/commit-dialog.js').then(({ openCommitChangesDialog }) => {
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
      btn.textContent = '更新中…';
      void pullCorpusRepo(corpusRepo)
        .catch((err) => {
          alert(`更新失败：${err?.message || '未知错误'}`);
        })
        .finally(() => {
          btn.disabled = false;
          btn.textContent = '↓ 更新项目';
        });
      return;
    }
    void pullProject();
  });

  document.getElementById('btn-local-refresh')?.addEventListener('click', () => {
    const btn = document.getElementById('btn-local-refresh');
    if (!btn) return;
    btn.disabled = true;
    btn.textContent = '⟳ 刷新中…';
    const finish = () => {
      btn.disabled = false;
      btn.textContent = '⟳ 本地刷新';
    };
    if (corpusRepo && onCorpusRefresh) {
      void Promise.resolve(onCorpusRefresh()).finally(finish);
      return;
    }
    loadIndex().finally(finish);
  });
}
