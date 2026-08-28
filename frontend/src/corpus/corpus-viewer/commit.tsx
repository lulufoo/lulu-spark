import type { ReactNode } from 'react';
import { state } from '../../host/state.ts';
import * as api from '../../host/api.ts';
import { renderKbComments } from '../corpus-comments.tsx';
import { applyKbHighlights } from './highlight.ts';
import { kbHidePendingBadge, kbPendingMsg, showKbReindexBtn } from './chrome.tsx';
import { renderToHtml } from '../../island.ts';

const REVERTABLE = new Set(['new', 'modified', 'deleted']);
const GROUPS = [
  { key: 'new', label: 'New' },
  { key: 'modified', label: 'Modified' },
  { key: 'renamed', label: 'Renamed' },
  { key: 'deleted', label: 'Deleted' },
  { key: 'conflicted', label: 'Conflict' },
] as const;

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

function paintFileList(fileList: HTMLElement, node: ReactNode) {
  fileList.innerHTML = renderToHtml(node);
}

function FileListGroups({ data }: { data: KbStatus }) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
      {data.ahead ? (
        <div className="commit-file-group">
          <div className="commit-file-group-title" style={{ color: '#0969da' }}>
            Ready to push ({data.ahead} local commits)
          </div>
          <div className="commit-file-item" style={{ background: '#ddf4ff', color: '#0550ae', display: 'block' }}>
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
                <span className="commit-file-item-path">{f}</span>
                {REVERTABLE.has(key) ? (
                  <button
                    type="button"
                    className="kb-revert-btn"
                    title={`Revert changes to ${f}`}
                    data-path={f}
                    data-type={key}
                  >
                    Revert
                  </button>
                ) : null}
              </div>
            ))}
          </div>
        );
      })}
    </div>
  );
}

function bindRevertButtons(fileList: HTMLElement) {
  fileList.querySelectorAll<HTMLButtonElement>('.kb-revert-btn').forEach((btn) => {
    const path = btn.dataset.path || '';
    const type = btn.dataset.type || '';
    btn.addEventListener('click', () => {
      void _kbRevertFile(path, type, btn);
    });
  });
}

export async function openKbCommitDialog() {
  const dialog = document.getElementById('kb-commit-dialog');
  const result = document.getElementById('kb-commit-result');
  const okBtn = document.getElementById('kb-btn-commit-ok') as HTMLButtonElement | null;
  const msgInput = document.getElementById('kb-commit-msg') as HTMLInputElement | null;
  if (!dialog || !result || !okBtn || !msgInput) return;

  result.textContent = '';
  result.style.color = '';
  msgInput.value = kbPendingMsg() || '';
  okBtn.disabled = false;
  okBtn.textContent = 'Commit';
  _resetRevertAllBtn();
  dialog.classList.add('open');

  await _refreshKbCommitFileList();
}

export async function _refreshKbCommitFileList() {
  const fileList = document.getElementById('kb-commit-file-list');
  const okBtn = document.getElementById('kb-btn-commit-ok') as HTMLButtonElement | null;
  const revertAllBtn = document.getElementById('kb-btn-revert-all') as HTMLButtonElement | null;
  if (!fileList || !okBtn) return;

  paintFileList(fileList, <div style={{ fontSize: 12, color: '#8c959f' }}>Loading…</div>);

  try {
    const data = (await api.fetchKbStatus(viewer().kbRepo)) as KbStatus;
    if (data.error) throw new Error(data.error);

    if (!data.total && !data.ahead) {
      paintFileList(
        fileList,
        <div style={{ fontSize: 13, color: '#8c959f', padding: '4px 0' }}>No changes to commit</div>,
      );
      okBtn.disabled = true;
      if (revertAllBtn) revertAllBtn.disabled = true;
    } else {
      paintFileList(fileList, <FileListGroups data={data} />);
      bindRevertButtons(fileList);
      okBtn.disabled = false;
      okBtn.textContent = data.total ? 'Commit' : 'Push';
      if (revertAllBtn) revertAllBtn.disabled = false;
    }
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    paintFileList(
      fileList,
      <div style={{ fontSize: 12, color: '#cf222e' }}>Failed to get status: {message}</div>,
    );
  }
}

export async function _kbRevertFile(path: string, type: string, btn: HTMLButtonElement) {
  btn.disabled = true;
  btn.textContent = 'Reverting…';
  try {
    const data = (await api.revertKbFile(viewer().kbRepo, path, type)) as { error?: string };
    if (data.error) throw new Error(data.error);
    await _refreshKbCommitFileList();
    const v = viewer();
    if (v.kbPath && path.startsWith('.knowledge_annotations')) {
      try {
        const ann = await api.fetchKbAnnotation(v.kbRepo, v.kbPath);
        v.annotation = ann;
        renderKbComments(ann);
        void applyKbHighlights();
      } catch {
        /* best-effort UI sync; revert already succeeded */
      }
    }
    const fileList = document.getElementById('kb-commit-file-list');
    const hasItems = fileList?.querySelector('.commit-file-item');
    if (!hasItems) {
      kbHidePendingBadge();
      setTimeout(closeKbCommitDialog, 800);
    }
  } catch (e) {
    btn.disabled = false;
    btn.textContent = 'Revert';
    btn.title = `Revert failed: ${e instanceof Error ? e.message : String(e)}`;
    btn.style.color = '#cf222e';
    btn.style.opacity = '1';
  }
}

let _revertAllConfirmTimer: ReturnType<typeof setTimeout> | null = null;

export function _resetRevertAllBtn() {
  const btn = document.getElementById('kb-btn-revert-all');
  if (!btn) return;
  if (_revertAllConfirmTimer != null) clearTimeout(_revertAllConfirmTimer);
  btn.textContent = 'Revert all changes';
  btn.classList.remove('confirm');
  (btn as HTMLButtonElement).disabled = false;
}

export async function _kbRevertAll(btn: HTMLButtonElement) {
  if (!btn.classList.contains('confirm')) {
    btn.classList.add('confirm');
    btn.textContent = '⚠ Revert all?';
    if (_revertAllConfirmTimer != null) clearTimeout(_revertAllConfirmTimer);
    _revertAllConfirmTimer = setTimeout(_resetRevertAllBtn, 3000);
    return;
  }
  if (_revertAllConfirmTimer != null) clearTimeout(_revertAllConfirmTimer);
  btn.disabled = true;
  btn.textContent = 'Reverting…';
  try {
    const data = (await api.revertKbFile(viewer().kbRepo)) as { error?: string };
    if (data.error) throw new Error(data.error);
    kbHidePendingBadge();
    _resetRevertAllBtn();
    await _refreshKbCommitFileList();
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
    _resetRevertAllBtn();
    const result = document.getElementById('kb-commit-result');
    if (result) {
      result.style.color = '#cf222e';
      result.textContent = `✗ Revert failed: ${e instanceof Error ? e.message : String(e)}`;
    }
  }
}

export function closeKbCommitDialog() {
  document.getElementById('kb-commit-dialog')?.classList.remove('open');
  const okBtn = document.getElementById('kb-btn-commit-ok');
  if (okBtn) okBtn.textContent = 'Commit';
  _resetRevertAllBtn();
}

export async function doKbCommit() {
  const btn = document.getElementById('kb-btn-commit-ok') as HTMLButtonElement | null;
  const result = document.getElementById('kb-commit-result');
  const msgInput = document.getElementById('kb-commit-msg') as HTMLInputElement | null;
  if (!btn || !result || !msgInput) return;
  const msg = msgInput.value.trim() || 'chore: update via viewer';
  btn.disabled = true;
  result.textContent = 'Committing…';
  result.style.color = '#8c959f';

  try {
    const data = (await api.commitKbFile(viewer().kbRepo, msg)) as { error?: string; stderr?: string };
    if (data.error) throw new Error(data.error + (data.stderr ? `\n${data.stderr}` : ''));
    result.style.color = '#1a7f37';
    result.textContent = '✓ Committed and pushed';
    kbHidePendingBadge();
    const repo = viewer().kbRepo;
    if (repo) showKbReindexBtn(repo);
    setTimeout(closeKbCommitDialog, 1500);
  } catch (e) {
    result.style.color = '#cf222e';
    result.textContent = `✗ ${e instanceof Error ? e.message : String(e)}`;
    btn.disabled = false;
  }
}
