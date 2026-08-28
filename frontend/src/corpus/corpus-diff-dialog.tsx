import type { ReactNode } from 'react';
import * as api from '../host/api.ts';
import { renderToHtml } from '../island.ts';

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

type DiffElements = {
  dialog: HTMLElement | null;
  box: HTMLElement | null;
  title: HTMLElement | null;
  fileList: HTMLElement | null;
  msg: HTMLTextAreaElement | null;
  result: HTMLElement | null;
  okBtn: HTMLButtonElement | null;
  cancelBtn: HTMLButtonElement | null;
  revertBtn: HTMLButtonElement | null;
};

let activeRepo = '';
let closeTimer: ReturnType<typeof setTimeout> | null = null;

function getElements(): DiffElements {
  return {
    dialog: document.getElementById('kb-diff-dialog'),
    box: document.getElementById('kb-diff-dialog-box'),
    title: document.getElementById('kb-diff-dialog-title'),
    fileList: document.getElementById('kb-diff-file-list'),
    msg: document.getElementById('kb-diff-msg') as HTMLTextAreaElement | null,
    result: document.getElementById('kb-diff-result'),
    okBtn: document.getElementById('btn-kb-diff-ok') as HTMLButtonElement | null,
    cancelBtn: document.getElementById('btn-kb-diff-cancel') as HTMLButtonElement | null,
    revertBtn: document.getElementById('btn-kb-diff-revert-all') as HTMLButtonElement | null,
  };
}

function repoShortName(repo: string) {
  return repo.split('/').pop() || repo;
}

function clearCloseTimer() {
  if (closeTimer) {
    clearTimeout(closeTimer);
    closeTimer = null;
  }
}

function emitUpdated() {
  window.dispatchEvent(new CustomEvent('kb-diff-updated', { detail: { repo: activeRepo } }));
}

function paintFileList(fileList: HTMLElement, node: ReactNode) {
  fileList.innerHTML = renderToHtml(node);
}

function FileListGroups({ data }: { data: DiffStatus }) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
      {data.ahead ? (
        <div className="commit-file-group">
          <div className="commit-file-group-title" style={{ color: '#0969da' }}>
            Ready to push ({data.ahead} local commits)
          </div>
          <div className="commit-file-item renamed" style={{ display: 'block' }}>
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
            {files.map((path) => (
              <div className={`commit-file-item ${key}`} key={path}>
                {path}
              </div>
            ))}
          </div>
        );
      })}
    </div>
  );
}

function renderGroups(data: DiffStatus) {
  const { fileList } = getElements();
  if (!fileList) return;
  paintFileList(fileList, <FileListGroups data={data} />);
}

async function refreshDialog() {
  const { fileList, okBtn, revertBtn, title } = getElements();
  if (!fileList || !okBtn || !revertBtn || !title) return;
  title.textContent = `✎ Local changes · ${repoShortName(activeRepo)}`;
  paintFileList(fileList, <div style={{ fontSize: 12, color: '#8c959f' }}>Loading…</div>);
  okBtn.disabled = true;
  revertBtn.disabled = true;

  try {
    const data = (await api.fetchKbStatus(activeRepo)) as DiffStatus | null;
    if (data?.error) throw new Error(data.error);
    if (!data?.total && !data?.ahead) {
      paintFileList(
        fileList,
        <div style={{ fontSize: 13, color: '#8c959f', padding: '4px 0' }}>No local changes</div>,
      );
      return;
    }
    renderGroups(data);
    okBtn.disabled = false;
    revertBtn.disabled = false;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    paintFileList(
      fileList,
      <div style={{ fontSize: 12, color: '#cf222e' }}>Failed to get status: {message}</div>,
    );
  }
}

export async function openKbDiffDialog(repo: string) {
  const { dialog, msg, result, okBtn } = getElements();
  if (!dialog || !msg || !result || !okBtn) return;
  activeRepo = repo;
  clearCloseTimer();
  msg.value = '';
  result.textContent = '';
  result.style.color = '';
  okBtn.textContent = 'Commit';
  dialog.classList.add('open');
  await refreshDialog();
}

export function closeKbDiffDialog() {
  const { dialog, result, okBtn } = getElements();
  clearCloseTimer();
  dialog?.classList.remove('open');
  if (result) {
    result.textContent = '';
    result.style.color = '';
  }
  if (okBtn) {
    okBtn.disabled = false;
    okBtn.textContent = 'Commit';
  }
}

async function handleCommit() {
  const { msg, result, okBtn, revertBtn } = getElements();
  if (!msg || !result || !okBtn || !revertBtn) return;
  okBtn.disabled = true;
  revertBtn.disabled = true;
  result.textContent = 'Committing…';
  result.style.color = '#8c959f';

  try {
    const data = (await api.commitKbFile(activeRepo, msg.value.trim() || 'chore: update via viewer')) as {
      error?: string;
    };
    if (data?.error) throw new Error(data.error);
    result.textContent = '✓ Committed and pushed';
    result.style.color = '#1a7f37';
    emitUpdated();
    closeTimer = setTimeout(() => closeKbDiffDialog(), 1500);
  } catch (error) {
    result.textContent = `Failed: ${error instanceof Error ? error.message : String(error)}`;
    result.style.color = '#cf222e';
    okBtn.disabled = false;
    revertBtn.disabled = false;
  }
}

async function handleRevertAll() {
  const { result, okBtn, revertBtn } = getElements();
  if (!result || !okBtn || !revertBtn) return;
  okBtn.disabled = true;
  revertBtn.disabled = true;
  result.textContent = 'Reverting…';
  result.style.color = '#8c959f';

  try {
    const data = (await api.revertKbFile(activeRepo)) as { error?: string };
    if (data?.error) throw new Error(data.error);
    result.textContent = '✓ Local changes discarded';
    result.style.color = '#1a7f37';
    emitUpdated();
    await refreshDialog();
    closeTimer = setTimeout(() => closeKbDiffDialog(), 1500);
  } catch (error) {
    result.textContent = `Failed: ${error instanceof Error ? error.message : String(error)}`;
    result.style.color = '#cf222e';
    okBtn.disabled = false;
    revertBtn.disabled = false;
  }
}

getElements().cancelBtn?.addEventListener('click', closeKbDiffDialog);
getElements().okBtn?.addEventListener('click', () => {
  void handleCommit();
});
getElements().revertBtn?.addEventListener('click', () => {
  void handleRevertAll();
});
getElements().dialog?.addEventListener('click', (event) => {
  if (event.target === getElements().dialog) closeKbDiffDialog();
});
