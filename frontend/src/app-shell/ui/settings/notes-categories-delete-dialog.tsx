import { useEffect, useSyncExternalStore } from 'react';
import { OverlayDismissButton } from '../../../shared/overlay-dismiss-button.tsx';
import { cancelDeleteProject, confirmDeleteProject } from '../../commands/settings/notes-categories-delete.ts';
import { notesCategoriesStore } from '../../state/settings/notes-categories.ts';

/** What the dialog tells the user will happen; the inbox can be renamed, so its name comes from the list. */
export function describeProjectDelete(title: string, count: number, inboxTitle: string): string {
  if (count <= 0) return `“${title}” has no documents. It will be deleted. This cannot be undone.`;
  const docs = count === 1 ? '1 document' : `${count} documents`;
  return `${docs} in “${title}” will be moved to “${inboxTitle}”, then “${title}” is deleted. This cannot be undone.`;
}

/** Confirm step for deleting a project: states the document count and where the documents go. */
export function NotesCategoriesDeleteDialog() {
  const snap = useSyncExternalStore(notesCategoriesStore.subscribe, notesCategoriesStore.getSnapshot);
  const row = snap.rows.find((r) => r.id === snap.deleteId);
  const inbox = snap.rows.find((r) => r.is_default);
  const open = Boolean(row && inbox);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape' || snap.busy) return;
      e.preventDefault();
      cancelDeleteProject();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, snap.busy]);

  return (
    <div
      id="notes-cat-delete-dialog"
      className={open ? 'open' : undefined}
      role="presentation"
      onClick={(e) => {
        if (e.target === e.currentTarget) cancelDeleteProject();
      }}
    >
      {row && inbox ? (
        <div
          id="notes-cat-delete-dialog-box"
          role="dialog"
          aria-modal="true"
          aria-labelledby="notes-cat-delete-dialog-title"
        >
          <div id="notes-cat-delete-dialog-header">
            <h3 id="notes-cat-delete-dialog-title">Delete project</h3>
            <OverlayDismissButton
              id="btn-notes-cat-delete-close"
              disabled={snap.busy}
              label="Close"
              onClick={() => cancelDeleteProject()}
            />
          </div>
          <div id="notes-cat-delete-dialog-body">
            <p id="notes-cat-delete-copy" className="notes-cat-confirm-copy">
              {describeProjectDelete(row.title || row.id, row.doc_count, inbox.title || inbox.id)}
            </p>
            <div id="notes-cat-delete-error" className="settings-result notes-cat-error">
              {snap.deleteError}
            </div>
            <div className="notes-cat-edit-actions">
              <button type="button" className="btn-settings-ghost" disabled={snap.busy} onClick={() => cancelDeleteProject()}>
                Cancel
              </button>
              <button
                id="btn-notes-cat-delete-confirm"
                type="button"
                className="sediment-kb-delete-btn notes-cat-delete-confirm"
                disabled={snap.busy}
                onClick={() => void confirmDeleteProject(snap.workspaceId)}
              >
                {snap.busy ? 'Deleting…' : 'Delete'}
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}
