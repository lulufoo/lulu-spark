import { useEffect, useState, useSyncExternalStore } from 'react';
import { getEntryId, state } from '../host/state.ts';
import * as api from '../host/api.ts';
import { createModuleStore } from '../shared/module-store.ts';

const openStore = createModuleStore(false);

export function openDeleteDialog() {
  openStore.set(true);
  document.getElementById('delete-dialog')?.classList.add('open');
}

export function closeDeleteDialog() {
  openStore.set(false);
  document.getElementById('delete-dialog')?.classList.remove('open');
}

export async function confirmDeleteDocument() {
  const entryId = getEntryId(state.viewer.entry);
  const data = await api.deleteEntry(entryId);
  if (data.error) throw new Error(data.error);
  const date = state.ui.activeDate;
  closeDeleteDialog();
  if (date) {
    window.location.replace(`#/workbench?date=${encodeURIComponent(date)}`);
  } else {
    window.location.replace('#/workbench');
  }
  document.dispatchEvent(new CustomEvent('cta:reload'));
}

export function DeleteDialog() {
  const open = useSyncExternalStore(openStore.subscribe, openStore.getSnapshot);
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
          Deletes all linked files (raw / distilled / trace / digest) and removes from index.json.{' '}
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
