import { useEffect, useState, useSyncExternalStore } from 'react';
import {
  closeKnowledgeTreeDelete,
  knowledgeTreeDeleteStore,
} from '../state/tree-menu.ts';

function emitDelete(path: string) {
  closeKnowledgeTreeDelete();
  document.querySelector('.knowledge-doc-sidebar')?.dispatchEvent(
    new CustomEvent('kb:tree-delete-confirm', {
      bubbles: true,
      detail: { path },
    }),
  );
}

export function KnowledgeTreeDeleteDialog() {
  const snap = useSyncExternalStore(
    knowledgeTreeDeleteStore.subscribe,
    knowledgeTreeDeleteStore.getSnapshot,
  );
  const [typed, setTyped] = useState('');
  const [copyLabel, setCopyLabel] = useState('Copy');

  useEffect(() => {
    if (!snap.open) return;
    setTyped('');
    setCopyLabel('Copy');
    queueMicrotask(() => document.getElementById('kb-tree-delete-confirm-input')?.focus());
  }, [snap.open]);

  if (!snap.open) return null;
  return (
    <div
      id="knowledge-tree-delete-dialog"
      className="open"
      onClick={(event) => {
        if (event.target === event.currentTarget) closeKnowledgeTreeDelete();
      }}
    >
      <div id="knowledge-tree-delete-dialog-box">
        <h3>⚠️ Delete {snap.isDir ? 'folder' : 'file'}</h3>
        <p>
          Delete <code>{snap.name}</code>
          {snap.isDir ? ' and everything inside' : ''}. <strong>Cannot be undone</strong>.
        </p>
        <div id="kb-tree-delete-confirm-row">
          Type <code>CONFIRM</code> to delete
          <button
            id="btn-kb-tree-copy-confirm"
            type="button"
            title="Copy CONFIRM"
            onClick={() => {
              void navigator.clipboard.writeText('CONFIRM');
              setCopyLabel('✓');
              setTimeout(() => setCopyLabel('Copy'), 1200);
            }}
          >
            {copyLabel}
          </button>
        </div>
        <input
          id="kb-tree-delete-confirm-input"
          type="text"
          placeholder="Enter CONFIRM here"
          autoComplete="off"
          value={typed}
          onChange={(e) => setTyped(e.target.value)}
        />
        <div id="knowledge-tree-delete-dialog-actions">
          <button type="button" onClick={() => closeKnowledgeTreeDelete()}>
            Cancel
          </button>
          <button
            type="button"
            id="btn-knowledge-tree-delete-ok"
            disabled={typed !== 'CONFIRM'}
            onClick={() => {
              if (typed !== 'CONFIRM') return;
              emitDelete(snap.path);
            }}
          >
            Confirm delete
          </button>
        </div>
      </div>
    </div>
  );
}
