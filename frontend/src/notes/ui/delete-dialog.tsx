import { useEffect, useState, useSyncExternalStore } from 'react';
import {
  closeDeleteDialog,
  confirmDeleteDocument,
  deleteOpenStore,
} from '../commands/delete-dialog.ts';

export { closeDeleteDialog, confirmDeleteDocument, openDeleteDialog } from '../commands/delete-dialog.ts';

export function DeleteDialog() {
  const open = useSyncExternalStore(deleteOpenStore.subscribe, deleteOpenStore.getSnapshot);
  const [typed, setTyped] = useState('');
  const [copyLabel, setCopyLabel] = useState('Copy');
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    setTyped('');
    setBusy(false);
    setCopyLabel('Copy');
    queueMicrotask(() => document.getElementById('delete-confirm-input')?.focus());
  }, [open]);

  async function onConfirm() {
    setBusy(true);
    try {
      await confirmDeleteDocument();
    } catch (e) {
      alert(`Delete failed: ${(e as Error).message}`);
      setBusy(false);
    }
  }

  return (
    <div id="delete-dialog" className={open ? 'open' : undefined}>
      <div id="delete-dialog-box">
        <h3>⚠️ Delete document</h3>
        <p id="delete-dialog-desc">
          Deletes all linked files (raw / digest) and removes from index.json.{' '}
          <strong>Cannot be undone</strong>.
        </p>
        <div id="delete-dialog-confirm-row">
          Type <code>CONFIRM</code> to delete
          <button
            id="btn-copy-confirm-word"
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
          id="delete-confirm-input"
          type="text"
          placeholder="Enter CONFIRM here"
          autoComplete="off"
          value={typed}
          onChange={(e) => setTyped(e.target.value)}
        />
        <div id="delete-dialog-actions">
          <button id="btn-delete-cancel" type="button" onClick={() => closeDeleteDialog()}>
            Cancel
          </button>
          <button
            id="btn-delete-confirm-ok"
            type="button"
            disabled={busy || typed !== 'CONFIRM'}
            onClick={() => void onConfirm()}
          >
            {busy ? 'Deleting…' : 'Confirm delete'}
          </button>
        </div>
      </div>
    </div>
  );
}
