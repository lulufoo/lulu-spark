import { useEffect, useSyncExternalStore } from 'react';
import { OverlayDismissButton } from '../../../shared/overlay-dismiss-button.tsx';
import {
  closeNotesCategoryEditor,
  saveNotesCategory,
  setNotesCategoryDraft,
} from '../../commands/settings/notes-categories.ts';
import { notesCategoriesStore } from '../../state/settings/notes-categories.ts';

/** Edit dialog for one project of the selected workspace; the list lives in `workspace-project-list.tsx`. */
export function NotesCategoriesEditHost() {
  const snap = useSyncExternalStore(
    notesCategoriesStore.subscribe,
    notesCategoriesStore.getSnapshot,
  );
  const selected = snap.rows.find((row) => row.id === snap.selectedId);

  useEffect(() => {
    if (!snap.editorOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape' || snap.busy) return;
      e.preventDefault();
      closeNotesCategoryEditor();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [snap.editorOpen, snap.busy]);

  useEffect(() => {
    if (!snap.editorOpen) return;
    queueMicrotask(() => document.getElementById('notes-cat-edit-title')?.focus());
  }, [snap.editorOpen, snap.selectedId]);

  const save = () => void saveNotesCategory(snap.workspaceId);

  return (
    <div
      id="notes-cat-edit-dialog"
      className={snap.editorOpen ? 'open' : undefined}
      role="presentation"
      onClick={(e) => {
        if (e.target === e.currentTarget && !snap.busy) closeNotesCategoryEditor();
      }}
    >
      {selected ? (
        <div
          id="notes-cat-edit-dialog-box"
          role="dialog"
          aria-modal="true"
          aria-labelledby="notes-cat-edit-dialog-title"
        >
          <div id="notes-cat-edit-dialog-header">
            <h3 id="notes-cat-edit-dialog-title">Edit project</h3>
            <OverlayDismissButton
              id="btn-notes-cat-edit-close"
              disabled={snap.busy}
              title="Close"
              onClick={() => closeNotesCategoryEditor()}
            />
          </div>
          <div id="notes-cat-edit-dialog-body">
            <div className="settings-field">
              <label htmlFor="notes-cat-edit-title">Title</label>
              <input
                id="notes-cat-edit-title"
                type="text"
                value={snap.draftTitle}
                autoComplete="off"
                spellCheck={false}
                disabled={snap.busy}
                onChange={(e) => setNotesCategoryDraft(e.target.value, snap.draftDescription)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    save();
                  }
                }}
              />
            </div>
            <div className="settings-field">
              <label htmlFor="notes-cat-edit-description">Description</label>
              <textarea
                id="notes-cat-edit-description"
                rows={3}
                value={snap.draftDescription}
                autoComplete="off"
                spellCheck={false}
                disabled={snap.busy}
                onChange={(e) => setNotesCategoryDraft(snap.draftTitle, e.target.value)}
              />
            </div>
            <div id="notes-cat-edit-error" className="settings-result notes-cat-error">
              {snap.error}
            </div>
            <div className="notes-cat-edit-actions">
              <span className="notes-cat-edit-actions-spacer" />
              <button
                type="button"
                className="btn-settings-ghost"
                disabled={snap.busy}
                onClick={() => closeNotesCategoryEditor()}
              >
                Cancel
              </button>
              <button
                id="btn-notes-cat-save"
                type="button"
                className="btn-settings-save"
                disabled={snap.busy || !snap.draftTitle.trim()}
                onClick={save}
              >
                {snap.busy ? 'Saving…' : 'Save'}
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}
