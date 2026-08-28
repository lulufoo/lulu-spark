import { useSyncExternalStore } from 'react';
import { flushSync } from 'react-dom';
import * as api from '../../host/api.ts';
import { createModuleStore } from '../../shared/module-store.ts';

const openStore = createModuleStore(false);

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

type FileListSnap =
  | { kind: 'empty' }
  | { kind: 'loading' }
  | { kind: 'none' }
  | { kind: 'error'; message: string }
  | { kind: 'groups'; data: DiffStatus };

const fileListStore = createModuleStore<FileListSnap>({ kind: 'empty' });

function publishFileList(next: FileListSnap) {
  flushSync(() => {
    fileListStore.set(next);
  });
}

const GROUPS = [
  { key: 'new', label: 'New' },
  { key: 'modified', label: 'Modified' },
  { key: 'renamed', label: 'Renamed' },
  { key: 'deleted', label: 'Deleted' },
  { key: 'conflicted', label: 'Conflict' },
] as const;

type DiffElements = {
  dialog: HTMLElement | null;
  title: HTMLElement | null;
  fileList: HTMLElement | null;
  msg: HTMLInputElement | null;
  result: HTMLElement | null;
  okBtn: HTMLButtonElement | null;
  revertBtn: HTMLButtonElement | null;
};

let activeRepo = '';
let closeTimer: ReturnType<typeof setTimeout> | null = null;

function getElements(): DiffElements {
  return {
    dialog: document.getElementById('kb-diff-dialog'),
    title: document.getElementById('kb-diff-dialog-title'),
    fileList: document.getElementById('kb-diff-file-list'),
    msg: document.getElementById('kb-diff-msg') as HTMLInputElement | null,
    result: document.getElementById('kb-diff-result'),
    okBtn: document.getElementById('btn-kb-diff-ok') as HTMLButtonElement | null,
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

async function refreshDialog() {
  const { fileList, okBtn, revertBtn, title } = getElements();
  if (!fileList || !okBtn || !revertBtn || !title) return;
  title.textContent = `✎ Local changes · ${repoShortName(activeRepo)}`;
  publishFileList({ kind: 'loading' });
  okBtn.disabled = true;
  revertBtn.disabled = true;

  try {
    const data = (await api.fetchKbStatus(activeRepo)) as DiffStatus | null;
    if (data?.error) throw new Error(data.error);
    if (!data?.total && !data?.ahead) {
      publishFileList({ kind: 'none' });
      return;
    }
    publishFileList({ kind: 'groups', data });
    okBtn.disabled = false;
    revertBtn.disabled = false;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    publishFileList({ kind: 'error', message });
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
  openStore.set(true);
  dialog.classList.add('open');
  await refreshDialog();
}

export function closeKbDiffDialog() {
  const { dialog, result, okBtn } = getElements();
  openStore.set(false);
  publishFileList({ kind: 'empty' });
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

async function handleKbDiffCommit() {
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

async function handleKbDiffRevertAll() {
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

function KbDiffFileList() {
  const snap = useSyncExternalStore(fileListStore.subscribe, fileListStore.getSnapshot);
  if (snap.kind === 'loading') {
    return <div style={{ fontSize: 12, color: '#8c959f' }}>Loading…</div>;
  }
  if (snap.kind === 'none') {
    return <div style={{ fontSize: 13, color: '#8c959f', padding: '4px 0' }}>No local changes</div>;
  }
  if (snap.kind === 'error') {
    return <div style={{ fontSize: 12, color: '#cf222e' }}>Failed to get status: {snap.message}</div>;
  }
  if (snap.kind === 'groups') {
    return <FileListGroups data={snap.data} />;
  }
  return null;
}

export function KbDiffDialog() {
  const open = useSyncExternalStore(openStore.subscribe, openStore.getSnapshot);

  return (
    <div
      id="kb-diff-dialog"
      className={open ? 'open' : undefined}
      onClick={(e) => {
        if (e.target === e.currentTarget) closeKbDiffDialog();
      }}
    >
      <div id="kb-diff-dialog-box">
        <div id="kb-diff-dialog-title-row">
          <h3 id="kb-diff-dialog-title">✎ Local changes</h3>
          <button
            id="btn-kb-diff-revert-all"
            type="button"
            onClick={() => {
              void handleKbDiffRevertAll();
            }}
          >
            Discard changes
          </button>
        </div>
        <div id="kb-diff-file-list">
          <KbDiffFileList />
        </div>
        <input
          id="kb-diff-msg"
          type="text"
          placeholder="Commit message (empty: chore: update via viewer)"
          autoComplete="off"
        />
        <div id="kb-diff-dialog-actions">
          <span id="kb-diff-result"></span>
          <button id="btn-kb-diff-cancel" type="button" onClick={() => closeKbDiffDialog()}>
            Cancel
          </button>
          <button
            id="btn-kb-diff-ok"
            type="button"
            onClick={() => {
              void handleKbDiffCommit();
            }}
          >
            Commit
          </button>
        </div>
      </div>
    </div>
  );
}
