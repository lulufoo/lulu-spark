import { useEffect, useSyncExternalStore } from 'react';
import { ReorderRows } from '../../../shared/reorder-rows.tsx';
import { OverlayDismissButton } from '../../../shared/overlay-dismiss-button.tsx';
import {
  closeNotesCategoryEditor,
  openNotesCategoryEditor,
  reorderNotesCategories,
  saveNotesCategory,
  setNotesCategoryDraft,
  setNotesCategoryDraftIcon,
} from '../../commands/settings/notes-categories.ts';
import { Icon } from '../../../shared/icons/icon.tsx';
import { DEFAULT_PROJECT_ICON_ID } from '../../../shared/icons/index.ts';
import { notesCategoriesStore } from '../../state/settings/notes-categories.ts';
import { IconPicker } from './icon-picker.tsx';

/** Project list of the selected workspace (whole row opens the dialog) and the edit dialog. */
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
    <>
      <p className="notes-cat-lead">
        Click a project to change its title or description; drag the handle to change the order. Existing documents keep their path.
      </p>
      {snap.error && !snap.editorOpen ? (
        <div id="notes-cat-list-error" className="settings-result notes-cat-error">
          {snap.error}
        </div>
      ) : null}
      <div id="notes-cat-list">
        <ReorderRows
          disabled={snap.busy}
          onReorder={(folders) => void reorderNotesCategories(snap.workspaceId, folders)}
          rows={snap.rows.map((row) => {
            const desc = row.description.trim();
            return {
              id: row.id,
              content: (
                <button
                  type="button"
                  className="notes-cat-item"
                  data-project={row.id}
                  disabled={snap.busy}
                  onClick={() => openNotesCategoryEditor(row.id)}
                >
                  <span className="notes-cat-item-main">
                    <span className="notes-cat-item-title-row">
                      <span className="notes-cat-item-title">{row.title || row.id}</span>
                      {row.is_inbox ? <span className="notes-cat-badge">Built-in</span> : null}
                    </span>
                    <span className={`notes-cat-item-desc${desc ? '' : ' is-empty'}`}>
                      {desc || 'No description'}
                    </span>
                  </span>
                  <span className="notes-cat-item-action">Edit</span>
                </button>
              ),
            };
          })}
        />
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
              <h3 id="notes-cat-edit-dialog-title">Edit project</h3>
              <OverlayDismissButton
                id="btn-notes-cat-edit-close"
                disabled={snap.busy}
                label="Close"
                onClick={() => closeNotesCategoryEditor()}
              />
            </div>
            <div id="notes-cat-edit-dialog-body">
              <div className="settings-field">
                <label htmlFor="notes-cat-edit-title">Title</label>
                <div className="icon-name-row">
                  <IconPicker
                    label="Project icon"
                    value={snap.draftIcon}
                    defaultIcon={<Icon id={DEFAULT_PROJECT_ICON_ID} size={18} />}
                    disabled={snap.busy}
                    onChange={setNotesCategoryDraftIcon}
                  />
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
    </>
  );
}
