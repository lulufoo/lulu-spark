import { useSyncExternalStore, type ReactNode } from 'react';
import * as api from '../../host/api.ts';
import { loadDiffStatus } from '../../host/state.ts';
import { renderToHtml } from '../../island.ts';
import { updateDiffInDOM } from '../cards.tsx';
import { escHtml } from '../../shared/utils.ts';
import { createModuleStore } from '../../shared/module-store.ts';

const openStore = createModuleStore(false);

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
  openStore.set(false);
  document.getElementById('md-commit-dialog')?.classList.remove('open');
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
  openStore.set(true);
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

export async function doMdCommit() {
  const btn = document.getElementById('md-btn-commit-ok') as HTMLButtonElement;
  const resultEl = document.getElementById('md-commit-dialog-result') as HTMLElement;
  const msg =
    (document.getElementById('md-commit-dialog-msg') as HTMLInputElement).value.trim() ||
    'update: edit via viewer';
  btn.disabled = true;
  resultEl.textContent = 'Committing…';
  resultEl.style.color = '#8c959f';
  try {
    const data = (await api.commitFiles(msg)) as { error?: string };
    if (data.error) throw new Error(data.error);
    resultEl.style.color = '#1a7f37';
    resultEl.textContent = '✓ Pushed!';
    await loadDiffStatus();
    updateDiffInDOM();
    hidePendingBadge();
    setTimeout(() => closeCommitDialog(), 1500);
  } catch (e) {
    resultEl.style.color = '#cf222e';
    resultEl.textContent = `✗ ${(e as Error).message}`;
    btn.disabled = false;
  }
}

export async function doMdRevertAll() {
  const btn = document.getElementById('md-btn-revert-all') as HTMLButtonElement;
  if (!btn.classList.contains('confirm')) {
    btn.classList.add('confirm');
    btn.textContent = 'Revert all changes?';
    setTimeout(() => {
      btn.classList.remove('confirm');
      btn.textContent = 'Revert all changes';
    }, 3000);
    return;
  }
  btn.disabled = true;
  try {
    const data = (await api.revertFile('', '')) as { error?: string };
    if (data.error) throw new Error(data.error);
    await loadDiffStatus();
    updateDiffInDOM();
    hidePendingBadge();
    closeCommitDialog();
  } catch (e) {
    const resultEl = document.getElementById('md-commit-dialog-result');
    if (resultEl) resultEl.textContent = `Revert failed: ${(e as Error).message}`;
    btn.disabled = false;
  } finally {
    btn.classList.remove('confirm');
    btn.textContent = 'Revert all changes';
  }
}

export async function doMdRevertFile(revertBtn: HTMLButtonElement) {
  const path = revertBtn.dataset.path;
  const type = revertBtn.dataset.type;
  revertBtn.disabled = true;
  try {
    const data = (await api.revertFile(path, type)) as { error?: string };
    if (data.error) throw new Error(data.error);
    await openCommitDialog();
  } catch (e) {
    revertBtn.disabled = false;
    const resultEl = document.getElementById('md-commit-dialog-result');
    if (resultEl) resultEl.textContent = `Revert failed: ${escHtml((e as Error).message)}`;
  }
}

export function MdCommitDialog() {
  const open = useSyncExternalStore(openStore.subscribe, openStore.getSnapshot);

  return (
    <div
      id="md-commit-dialog"
      className={open ? 'open' : undefined}
      onClick={(e) => {
        if (e.target === e.currentTarget) closeCommitDialog();
      }}
    >
      <div id="md-commit-dialog-box">
        <div id="md-commit-dialog-title">
          <h3>● Commit changes</h3>
          <button id="md-btn-revert-all" type="button" onClick={() => void doMdRevertAll()}>
            Revert all changes
          </button>
        </div>
        <div
          id="md-commit-file-list"
          onClick={(e) => {
            const revertBtn = (e.target as HTMLElement).closest?.('.kb-revert-btn');
            if (!revertBtn) return;
            void doMdRevertFile(revertBtn as HTMLButtonElement);
          }}
        ></div>
        <input id="md-commit-dialog-msg" type="text" placeholder="update: edit via viewer" />
        <div id="md-commit-dialog-actions">
          <span id="md-commit-dialog-result"></span>
          <button
            id="md-btn-commit-cancel"
            type="button"
            className="md-header-btn"
            onClick={() => closeCommitDialog()}
          >
            Cancel
          </button>
          <button
            id="md-btn-commit-ok"
            type="button"
            className="md-header-btn primary"
            onClick={() => void doMdCommit()}
          >
            Commit
          </button>
        </div>
      </div>
    </div>
  );
}
