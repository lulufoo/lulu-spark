import type { ReactNode } from 'react';
import * as api from '../host/api.ts';
import { showToast } from '../toast.tsx';
import { renderToHtml } from '../island.ts';

const HEADER_LABEL_IDLE = '↑ Commit changes';
const HEADER_LABEL_CHECKING = 'Checking…';
const CLOSE_DELAY_MS = 500;

const GROUPS = [
  { key: 'new', label: 'New' },
  { key: 'modified', label: 'Modified' },
  { key: 'renamed', label: 'Renamed' },
  { key: 'deleted', label: 'Deleted' },
  { key: 'conflicted', label: 'Conflict' },
] as const;

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

function paintFileList(fileList: HTMLElement, node: ReactNode) {
  fileList.innerHTML = renderToHtml(node);
}

export async function openCommitChangesDialog() {
  const headerBtn = document.getElementById('btn-push-index') as HTMLButtonElement;
  headerBtn.disabled = true;
  headerBtn.textContent = HEADER_LABEL_CHECKING;

  const fileList = document.getElementById('commit-changes-file-list') as HTMLElement;
  const result = document.getElementById('commit-changes-result') as HTMLElement;
  paintFileList(
    fileList,
    <div style={{ fontSize: 12, color: '#8c959f' }}>Loading…</div>,
  );
  result.textContent = '';
  result.style.color = '';
  (document.getElementById('commit-changes-msg') as HTMLInputElement).value = '';
  (document.getElementById('btn-commit-changes-ok') as HTMLButtonElement).disabled = false;
  document.getElementById('commit-changes-dialog')?.classList.add('open');

  try {
    const data = (await api.fetchDiffStatus()) as DiffStatus | null;
    if (!data) throw new Error('Could not get status');
    if (data.error) throw new Error(data.error);

    if (!data.total && !data.ahead) {
      paintFileList(
        fileList,
        <div style={{ fontSize: 13, color: '#8c959f', padding: '4px 0' }}>
          No changes to commit or push
        </div>,
      );
      (document.getElementById('btn-commit-changes-ok') as HTMLButtonElement).disabled = true;
    } else {
      paintFileList(
        fileList,
        <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
          {data.ahead ? (
            <div className="commit-file-group">
              <div className="commit-file-group-title" style={{ color: '#0969da' }}>
                Ready to push ({data.ahead} local commits)
              </div>
              <div className="commit-file-item" style={{ background: '#ddf4ff', color: '#0550ae' }}>
                {data.ahead} local commit(s) not yet pushed
              </div>
            </div>
          ) : null}
          {GROUPS.map(({ key, label }) => {
            const files = data[key];
            if (!files?.length) return null;
            return (
              <div className="commit-file-group" key={key}>
                <div className="commit-file-group-title">
                  {label}（{files.length}）
                </div>
                {files.map((f) => (
                  <div className={`commit-file-item ${key}`} key={f}>
                    {f}
                  </div>
                ))}
              </div>
            );
          })}
        </div>,
      );
      (document.getElementById('btn-commit-changes-ok') as HTMLButtonElement).textContent = data.total
        ? 'Commit'
        : 'Push';
    }
  } catch (err) {
    const e = err as Error;
    paintFileList(
      fileList,
      <div style={{ fontSize: 12, color: '#cf222e' }}>Failed to get status: {e.message}</div>,
    );
  } finally {
    headerBtn.disabled = false;
    headerBtn.textContent = HEADER_LABEL_IDLE;
  }
}

function closeCommitChangesDialog() {
  clearCommitCloseTimer();
  document.getElementById('commit-changes-dialog')?.classList.remove('open');
  const ok = document.getElementById('btn-commit-changes-ok') as HTMLButtonElement | null;
  if (ok) ok.textContent = 'Commit';
  const headerBtn = document.getElementById('btn-push-index') as HTMLButtonElement | null;
  if (headerBtn) {
    headerBtn.disabled = false;
    headerBtn.textContent = HEADER_LABEL_IDLE;
  }
}

function doCommitChanges() {
  const msg = (document.getElementById('commit-changes-msg') as HTMLInputElement).value.trim();
  clearCommitCloseTimer();
  commitCloseTimer = setTimeout(() => {
    commitCloseTimer = null;
    closeCommitChangesDialog();

    void api.commitFiles(msg || 'chore: update via viewer')
      .then((data: { info?: string }) => {
        const successMsg = data?.info === 'nothing to commit'
          ? '✓ Pushed'
          : '✓ Committed and pushed';
        showToast(successMsg, 'success');
      })
      .catch((e: Error) => {
        showToast(`Commit failed: ${e.message}`, 'error');
      });
  }, CLOSE_DELAY_MS);
}

document.getElementById('btn-commit-changes-cancel')?.addEventListener('click', closeCommitChangesDialog);
document.getElementById('btn-commit-changes-ok')?.addEventListener('click', doCommitChanges);
document.getElementById('commit-changes-dialog')?.addEventListener('click', (e) => {
  if (e.target === document.getElementById('commit-changes-dialog')) closeCommitChangesDialog();
});
