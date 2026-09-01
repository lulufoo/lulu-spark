import { useEffect, useSyncExternalStore } from 'react';
import {
  askNotesCategoryDelete,
  cancelNotesCategoryDelete,
  closeNotesCategoryEditor,
  openNotesCategoryEditor,
  removeNotesCategory,
  saveNotesCategory,
  setNotesCategoryDraft,
} from '../../commands/settings/notes-categories.ts';
import { notesCategoriesStore } from '../../state/settings/notes-categories.ts';

export function NotesCategoriesEditHost() {
  const snap = useSyncExternalStore(
    notesCategoriesStore.subscribe,
    notesCategoriesStore.getSnapshot,
  );
  const selected = snap.rows.find((row) => row.id === snap.selectedId);
  const inbox = selected?.id === 'inbox';

  useEffect(() => {
    if (!snap.editorOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape' || snap.busy) return;
      e.preventDefault();
      if (snap.confirmDelete) cancelNotesCategoryDelete();
      else closeNotesCategoryEditor();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [snap.editorOpen, snap.confirmDelete, snap.busy]);

  useEffect(() => {
    if (!snap.editorOpen || snap.confirmDelete) return;
    queueMicrotask(() => document.getElementById('notes-cat-edit-title')?.focus());
  }, [snap.editorOpen, snap.confirmDelete, snap.selectedId]);

  return (
    <>
      <p className="notes-cat-lead">
        Click a category to change its title or description. Existing notes keep their path.
      </p>
      {snap.error && !snap.editorOpen ? (
        <div id="notes-cat-list-error" className="settings-result notes-cat-error">
          {snap.error}
        </div>
      ) : null}
      <div id="notes-cat-list">
        {snap.rows.map((row) => {
          const desc = row.description.trim();
          return (
            <button
              key={row.id}
              type="button"
              className="notes-cat-item"
              disabled={snap.busy}
              onClick={() => openNotesCategoryEditor(row.id)}
            >
              <span className="notes-cat-item-main">
                <span className="notes-cat-item-title-row">
                  <span className="notes-cat-item-title">{row.title || row.id}</span>
                  {row.id === 'inbox' ? <span className="notes-cat-badge">Built-in</span> : null}
                </span>
                <span className={`notes-cat-item-desc${desc ? '' : ' is-empty'}`}>
                  {desc || 'No description'}
                </span>
              </span>
              <span className="notes-cat-item-action">Edit</span>
            </button>
          );
        })}
      </div>
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
              <h3 id="notes-cat-edit-dialog-title">
                {snap.confirmDelete ? 'Remove category' : 'Edit category'}
              </h3>
              <button
                type="button"
                id="btn-notes-cat-edit-close"
                disabled={snap.busy}
                title="Close"
                onClick={() => closeNotesCategoryEditor()}
              >
                ✕
              </button>
            </div>
            {snap.confirmDelete ? (
              <div id="notes-cat-edit-dialog-body">
                <p className="notes-cat-confirm-copy">
                  Remove <strong>{selected.title || selected.id}</strong> from the switch list?
                  Existing notes stay on disk.
                </p>
                <div id="notes-cat-edit-error" className="settings-result notes-cat-error">
                  {snap.error}
                </div>
                <div className="notes-cat-edit-actions">
                  <button
                    type="button"
                    className="btn-settings-ghost"
                    disabled={snap.busy}
                    onClick={() => cancelNotesCategoryDelete()}
                  >
                    Back
                  </button>
                  <button
                    id="btn-notes-cat-delete"
                    type="button"
                    className="sediment-kb-delete-btn"
                    disabled={snap.busy}
                    onClick={() => void removeNotesCategory(selected.id)}
                  >
                    {snap.busy ? 'Removing…' : 'Remove category'}
                  </button>
                </div>
              </div>
            ) : (
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
                        void saveNotesCategory();
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
                  {inbox ? (
                    <span className="notes-cat-edit-actions-spacer" />
                  ) : (
                    <button
                      type="button"
                      className="sediment-kb-delete-btn"
                      disabled={snap.busy}
                      onClick={() => askNotesCategoryDelete()}
                    >
                      Delete
                    </button>
                  )}
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
                    onClick={() => void saveNotesCategory()}
                  >
                    {snap.busy ? 'Saving…' : 'Save'}
                  </button>
                </div>
              </div>
            )}
          </div>
        ) : null}
      </div>
    </>
  );
}
