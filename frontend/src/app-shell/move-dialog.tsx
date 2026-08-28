import { useEffect, useState, useSyncExternalStore } from 'react';
import * as api from '../host/api.ts';
import { createModuleStore } from '../shared/module-store.ts';

type GhOpsPanel = 'move' | 'delete';
type ResultTone = '' | 'ok' | 'warn' | 'err' | 'busy';

const openStore = createModuleStore(false);

const TONE: Record<ResultTone, string> = {
  '': '',
  ok: '#1a7f37',
  warn: '#e09b00',
  err: '#cf222e',
  busy: '#57606a',
};

export function openMoveDocDialog() {
  openStore.set(true);
  document.getElementById('move-doc-dialog')?.classList.add('open');
}

export function closeMoveDocDialog() {
  openStore.set(false);
  document.getElementById('move-doc-dialog')?.classList.remove('open');
}

export async function doMoveDoc(
  srcUrl: string,
  dstUrl: string,
  setResult: (tone: ResultTone, text: string) => void,
) {
  if (!srcUrl || !dstUrl) {
    setResult('err', 'Enter both URLs');
    return;
  }
  setResult('busy', 'Running gh api…');
  try {
    const data = await api.ghMove(srcUrl, dstUrl);
    if (!data.ok || data.error) throw new Error(data.error || 'failed');
    if (data.warn) {
      const movedInfo = data.moved !== undefined ? ` (${data.moved} files moved)` : '';
      setResult('warn', `⚠ ${data.warn}${movedInfo}`);
      return;
    }
    const movedInfo = data.moved !== undefined ? ` (${data.moved} files total)` : '';
    setResult('ok', `✓ Moved to ${data.dst_path}${movedInfo}`);
    document.dispatchEvent(new CustomEvent('cta:reload'));
    setTimeout(closeMoveDocDialog, 2000);
  } catch (e) {
    setResult('err', `✗ ${(e as Error).message}`);
  }
}

export async function doDeleteDoc(
  url: string,
  setResult: (tone: ResultTone, text: string) => void,
) {
  if (!url) {
    setResult('err', 'Enter URL');
    return;
  }
  setResult('busy', 'Running gh api…');
  try {
    const data = await api.ghDelete(url);
    if (!data.ok || data.error) throw new Error(data.error || 'failed');
    if (data.warn) {
      const deletedInfo = data.deleted !== undefined ? ` (${data.deleted} files deleted)` : '';
      setResult('warn', `⚠ ${data.warn}${deletedInfo}`);
      return;
    }
    const deletedInfo = data.deleted !== undefined ? ` (${data.deleted} files total)` : '';
    setResult('ok', `✓ Deleted${deletedInfo}`);
    document.dispatchEvent(new CustomEvent('cta:reload'));
    setTimeout(closeMoveDocDialog, 2000);
  } catch (e) {
    setResult('err', `✗ ${(e as Error).message}`);
  }
}

export function MoveDocDialog() {
  const open = useSyncExternalStore(openStore.subscribe, openStore.getSnapshot);
  const [panel, setPanel] = useState<GhOpsPanel>('move');
  const [srcUrl, setSrcUrl] = useState('');
  const [dstUrl, setDstUrl] = useState('');
  const [deleteUrl, setDeleteUrl] = useState('');
  const [moveResult, setMoveResult] = useState({ tone: '' as ResultTone, text: '' });
  const [deleteResult, setDeleteResult] = useState({ tone: '' as ResultTone, text: '' });
  const [moveBusy, setMoveBusy] = useState(false);
  const [deleteBusy, setDeleteBusy] = useState(false);

  function resetDeleteColumn() {
    setDeleteUrl('');
    setDeleteResult({ tone: '', text: '' });
    setDeleteBusy(false);
  }

  useEffect(() => {
    if (!open) return;
    setSrcUrl('');
    setDstUrl('');
    setMoveResult({ tone: '', text: '' });
    setMoveBusy(false);
    resetDeleteColumn();
    setPanel('move');
    queueMicrotask(() => document.getElementById('move-src-url')?.focus());
  }, [open]);

  function switchGhOpsPanel(next: GhOpsPanel) {
    setPanel(next);
    queueMicrotask(() => {
      if (next === 'move') document.getElementById('move-src-url')?.focus();
      else document.getElementById('delete-url')?.focus();
    });
  }

  async function onMove() {
    setMoveBusy(true);
    try {
      await doMoveDoc(srcUrl.trim(), dstUrl.trim(), (tone, text) => {
        setMoveResult({ tone, text });
      });
    } finally {
      setMoveBusy(false);
    }
  }

  async function onDelete() {
    setDeleteBusy(true);
    try {
      await doDeleteDoc(deleteUrl.trim(), (tone, text) => {
        setDeleteResult({ tone, text });
      });
    } finally {
      setDeleteBusy(false);
    }
  }

  return (
    <div
      id="move-doc-dialog"
      className={open ? 'open' : undefined}
      onClick={(e) => {
        if (e.target === e.currentTarget) closeMoveDocDialog();
      }}
    >
      <div id="move-doc-dialog-box">
        <div id="move-doc-dialog-header">
          <span>↗ GitHub</span>
          <button id="btn-move-doc-close" type="button" title="Close" onClick={() => closeMoveDocDialog()}>
            ✕
          </button>
        </div>
        <div id="move-doc-dialog-body">
          <nav id="gh-ops-nav">
            <button
              type="button"
              className={`gh-ops-nav-item${panel === 'move' ? ' active' : ''}`}
              data-panel="move"
              onClick={() => switchGhOpsPanel('move')}
            >
              Move
            </button>
            <button
              type="button"
              className={`gh-ops-nav-item${panel === 'delete' ? ' active' : ''}`}
              data-panel="delete"
              onClick={() => switchGhOpsPanel('delete')}
            >
              Delete
            </button>
          </nav>
          <div id="gh-ops-panels">
            <div id="gh-ops-panel-move" className={`gh-ops-panel${panel === 'move' ? ' active' : ''}`}>
              <div className="gh-ops-panel-title">Move</div>
              <div className="gh-ops-field">
                <label htmlFor="move-src-url">Source URL (blob link for files, tree link for directories)</label>
                <input
                  id="move-src-url"
                  type="text"
                  placeholder="blob: …/blob/main/dir/file.md   or   tree: …/tree/main/dir"
                  autoComplete="off"
                  spellCheck={false}
                  value={srcUrl}
                  onChange={(e) => setSrcUrl(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') document.getElementById('move-dst-url')?.focus();
                  }}
                />
              </div>
              <div className="gh-ops-field">
                <label htmlFor="move-dst-url">
                  Target directory URL (tree directory link, or blob file link for its parent directory)
                </label>
                <input
                  id="move-dst-url"
                  type="text"
                  placeholder="tree: …/tree/main/subdir   or   blob: …/blob/main/subdir/file.md"
                  autoComplete="off"
                  spellCheck={false}
                  value={dstUrl}
                  onChange={(e) => setDstUrl(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') void onMove();
                    if (e.key === 'Escape') closeMoveDocDialog();
                  }}
                />
              </div>
              <div id="move-doc-result" className="gh-ops-result" style={{ color: TONE[moveResult.tone] }}>
                {moveResult.text}
              </div>
              <div className="gh-ops-panel-actions">
                <button
                  id="btn-move-doc-ok"
                  type="button"
                  className="btn-gh-ops-primary"
                  disabled={moveBusy}
                  onClick={() => void onMove()}
                >
                  {moveBusy ? 'Moving…' : 'Confirm move'}
                </button>
              </div>
            </div>
            <div id="gh-ops-panel-delete" className={`gh-ops-panel${panel === 'delete' ? ' active' : ''}`}>
              <div className="gh-ops-panel-title">Delete</div>
              <div className="gh-ops-field">
                <label htmlFor="delete-url">URL (blob link for files, tree link for directories)</label>
                <input
                  id="delete-url"
                  type="text"
                  placeholder="blob: …/blob/main/dir/file.md   or   tree: …/tree/main/dir"
                  autoComplete="off"
                  spellCheck={false}
                  value={deleteUrl}
                  onChange={(e) => setDeleteUrl(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') void onDelete();
                    if (e.key === 'Escape') closeMoveDocDialog();
                  }}
                />
              </div>
              <div id="delete-doc-result" className="gh-ops-result" style={{ color: TONE[deleteResult.tone] }}>
                {deleteResult.text}
              </div>
              <div className="gh-ops-panel-actions gh-ops-delete-actions">
                <button
                  id="btn-delete-doc-ok"
                  type="button"
                  className="btn-gh-ops-danger"
                  disabled={deleteBusy}
                  onClick={() => void onDelete()}
                >
                  {deleteBusy ? 'Deleting…' : 'Confirm delete'}
                </button>
              </div>
            </div>
          </div>
        </div>
        <div id="move-doc-dialog-footer">
          <button type="button" id="btn-move-doc-cancel" onClick={() => closeMoveDocDialog()}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
