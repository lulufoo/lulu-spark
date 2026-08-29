import * as api from '../../host/api.ts';
import { openKbDiffDialog } from '../../knowledge/ui/diff-dialog.tsx';
import { errMessage, type HeaderSyncDeps } from '../state/types.ts';

let knowledgeRepo = '';
let onKnowledgeRefresh: (() => void | Promise<void>) | null = null;

export function setHeaderSyncKnowledgeContext(
  repo: string,
  onRefresh?: (() => void | Promise<void>) | null,
) {
  knowledgeRepo = repo || '';
  onKnowledgeRefresh = typeof onRefresh === 'function' ? onRefresh : null;
}

export function clearHeaderSyncKnowledgeContext() {
  knowledgeRepo = '';
  onKnowledgeRefresh = null;
}

async function pullKnowledgeRepo(repo: string) {
  await api.reindexKbRepo(repo);
  for (let i = 0; i < 120; i++) {
    await new Promise((r) => setTimeout(r, 2000));
    const status = (await api.getReindexStatus()) as { status?: string };
    if (status?.status !== 'running') break;
  }
  if (onKnowledgeRefresh) await onKnowledgeRefresh();
}

/**
 * @param {{ pullProject: () => Promise<void>, loadIndex: () => Promise<void>, openWorkbenchCommit?: () => void | Promise<void> }} deps
 */
export function initHeaderSync({ pullProject, loadIndex, openWorkbenchCommit }: HeaderSyncDeps) {
  const openWorkbench = openWorkbenchCommit ?? (() =>
    import('./commit-dialog.ts').then(({ openCommitChangesDialog }) => {
      void openCommitChangesDialog();
    }));

  document.getElementById('btn-push-index')?.addEventListener('click', () => {
    if (knowledgeRepo) {
      void openKbDiffDialog(knowledgeRepo);
      return;
    }
    void openWorkbench();
  });

  document.getElementById('btn-pull')?.addEventListener('click', () => {
    const btn = document.getElementById('btn-pull') as HTMLButtonElement | null;
    if (!btn) return;
    if (knowledgeRepo) {
      btn.disabled = true;
      btn.textContent = 'Updating…';
      void pullKnowledgeRepo(knowledgeRepo)
        .catch((err) => {
          alert(`Update failed: ${errMessage(err, 'Unknown error')}`);
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
    const btn = document.getElementById('btn-local-refresh') as HTMLButtonElement | null;
    if (!btn) return;
    btn.disabled = true;
    btn.textContent = '⟳ Refreshing…';
    const finish = () => {
      btn.disabled = false;
      btn.textContent = '⟳ Refresh local';
    };
    if (knowledgeRepo && onKnowledgeRefresh) {
      void Promise.resolve(onKnowledgeRefresh()).finally(finish);
      return;
    }
    loadIndex().finally(finish);
  });
}
