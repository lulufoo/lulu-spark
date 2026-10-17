import { useEffect, useSyncExternalStore } from 'react';
import { OverlayDismissButton } from '../../../shared/overlay-dismiss-button.tsx';
import {
  askNotesCategoryDisable,
  cancelNotesCategoryDisable,
  closeNotesCategoryEditor,
  disableNotesCategory,
  enableNotesCategory,
  openNotesCategoryEditor,
  saveNotesCategory,
  setNotesCategoryDraft,
} from '../../commands/settings/notes-categories.ts';
import { notesCategoriesStore } from '../../state/settings/notes-categories.ts';

/**
 * Project list of the selected workspace (whole row opens the dialog), the edit dialog with its
 * Disable step, and the collapsed Disabled list.
 */
export function NotesCategoriesEditHost() {
  const snap = useSyncExternalStore(
    notesCategoriesStore.subscribe,
    notesCategoriesStore.getSnapshot,
  );
  const selected = snap.rows.find((row) => row.id === snap.selectedId);
  const inbox = selected?.id === 'inbox';
  const confirming = Boolean(selected) && snap.confirmDisableId === selected?.id;

  useEffect(() => {
    if (!snap.editorOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape' || snap.busy) return;
      e.preventDefault();
      if (confirming) cancelNotesCategoryDisable();
      else closeNotesCategoryEditor();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [snap.editorOpen, confirming, snap.busy]);

  useEffect(() => {
    if (!snap.editorOpen || confirming) return;
    queueMicrotask(() => document.getElementById('notes-cat-edit-title')?.focus());
  }, [snap.editorOpen, confirming, snap.selectedId]);

  const save = () => void saveNotesCategory(snap.workspaceId);

  return (
    <>
      <p className="notes-cat-lead">
        Click a project to change its title or description. Existing documents keep their path.
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
              data-project={row.id}
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
        {snap.disabled.length ? (
          <details id="notes-projects-disabled" className="notes-disabled">
            <summary className="notes-cat-lead">Disabled ({snap.disabled.length})</summary>
            {snap.disabled.map((row) => (
              <button
                key={row.id}
                type="button"
                className="notes-cat-item"
                data-project-disabled={row.id}
                disabled={snap.busy}
                onClick={() => void enableNotesCategory(snap.workspaceId, row.id)}
              >
                <span className="notes-cat-item-main">
                  <span className="notes-cat-item-title">{row.title || row.id}</span>
                </span>
                <span className="notes-cat-item-action">Enable</span>
              </button>
            ))}
          </details>
        ) : null}
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
                {confirming ? 'Disable project' : 'Edit project'}
              </h3>
              <OverlayDismissButton
                id="btn-notes-cat-edit-close"
                disabled={snap.busy}
                title="Close"
                onClick={() => closeNotesCategoryEditor()}
              />
            </div>
            {confirming ? (
              <div id="notes-cat-edit-dialog-body">
                <p className="notes-cat-confirm-copy">
                  Disable <strong>{selected.title || selected.id}</strong>? It is hidden from lists and
                  search. Its documents are kept, the folder stays reserved, and you can enable it
                  again from the Disabled list.
                </p>
                <div id="notes-cat-edit-error" className="settings-result notes-cat-error">
                  {snap.error}
                </div>
                <div className="notes-cat-edit-actions">
                  <button
                    type="button"
                    className="btn-settings-ghost"
                    disabled={snap.busy}
                    onClick={() => cancelNotesCategoryDisable()}
                  >
                    Back
                  </button>
                  <button
                    id="btn-notes-cat-disable"
                    type="button"
                    className="sediment-kb-delete-btn"
                    disabled={snap.busy}
                    onClick={() => void disableNotesCategory(snap.workspaceId, selected.id)}
                  >
                    {snap.busy ? 'Disabling…' : 'Disable project'}
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
                  {inbox ? (
                    <span className="notes-cat-edit-actions-spacer" />
                  ) : (
                    <button
                      type="button"
                      className="sediment-kb-delete-btn"
                      disabled={snap.busy}
                      onClick={() => askNotesCategoryDisable(selected.id)}
                    >
                      Disable
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
                    onClick={save}
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
