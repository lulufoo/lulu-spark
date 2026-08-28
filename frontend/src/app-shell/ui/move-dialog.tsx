import { useEffect, useState, useSyncExternalStore } from 'react';
import {
  closeMoveDocDialog,
  doDeleteDoc,
  doMoveDoc,
  moveDocOpenStore,
  type ResultTone,
} from '../commands/move-dialog.ts';

export { closeMoveDocDialog, openMoveDocDialog } from '../commands/move-dialog.ts';

type GhOpsPanel = 'move' | 'delete';

const TONE: Record<ResultTone, string> = {
  '': '',
  ok: '#1a7f37',
  warn: '#e09b00',
  err: '#cf222e',
  busy: '#57606a',
};

export function MoveDocDialog() {
  const open = useSyncExternalStore(moveDocOpenStore.subscribe, moveDocOpenStore.getSnapshot);
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
