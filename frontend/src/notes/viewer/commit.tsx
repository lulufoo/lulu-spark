import type { ReactNode } from 'react';
import * as api from '../../host/api.ts';
import { renderToHtml } from '../../island.ts';

const GROUPS = [
  { key: 'new', label: 'New' },
  { key: 'modified', label: 'Modified' },
  { key: 'deleted', label: 'Deleted' },
  { key: 'renamed', label: 'Renamed' },
  { key: 'conflicted', label: 'Conflict' },
] as const;

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

function paintFileList(fileList: HTMLElement, node: ReactNode) {
  fileList.innerHTML = renderToHtml(node);
}

export function showPendingBadge() {
  const btn = document.getElementById('btn-panel-commit');
  if (btn) btn.style.display = '';
}

export function hidePendingBadge() {
  const btn = document.getElementById('btn-panel-commit');
  if (btn) btn.style.display = 'none';
}

export function closeCommitDialog() {
  (document.getElementById('md-commit-dialog') as HTMLElement).classList.remove('open');
}

export async function openCommitDialog() {
  const dialog = document.getElementById('md-commit-dialog') as HTMLElement;
  const fileList = document.getElementById('md-commit-file-list') as HTMLElement;
  const msgInput = document.getElementById('md-commit-dialog-msg') as HTMLInputElement;
  const resultEl = document.getElementById('md-commit-dialog-result') as HTMLElement;
  const okBtn = document.getElementById('md-btn-commit-ok') as HTMLButtonElement;

  msgInput.value = '';
  resultEl.textContent = '';
  resultEl.style.color = '';
  paintFileList(fileList, <div style={{ fontSize: 12, color: '#8c959f' }}>Loading…</div>);
  okBtn.disabled = false;
  dialog.classList.add('open');

  try {
    const data = (await api.fetchDiffStatus()) as DiffStatus | null;
    if (!data) throw new Error('Could not get status');
    if (data.error) throw new Error(data.error);

    if (!data.total && !data.ahead) {
      paintFileList(
        fileList,
        <div style={{ fontSize: 13, color: '#8c959f', padding: '4px 0' }}>No changes to commit</div>,
      );
      okBtn.disabled = true;
      return;
    }

    paintFileList(
      fileList,
      <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
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
                  <span>{f}</span>
                  <button className="kb-revert-btn" data-path={f} data-type={key}>
                    Revert
                  </button>
                </div>
              ))}
            </div>
          );
        })}
      </div>,
    );
  } catch (err) {
    const e = err as Error;
    paintFileList(
      fileList,
      <div style={{ fontSize: 12, color: '#cf222e' }}>Failed to get status: {e.message}</div>,
    );
  }
}
