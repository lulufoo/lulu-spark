import { useSyncExternalStore } from 'react';
import {
  askNotesCategoryDisable,
  cancelNotesCategoryDisable,
  disableNotesCategory,
  enableNotesCategory,
  openNotesCategoryEditor,
} from '../../commands/settings/notes-categories.ts';
import { notesCategoriesStore } from '../../state/settings/notes-categories.ts';

/** Projects of the selected workspace: Edit and Disable per row, then a collapsed Disabled list. */
export function WorkspaceProjectList() {
  const snap = useSyncExternalStore(
    notesCategoriesStore.subscribe,
    notesCategoriesStore.getSnapshot,
  );
  const { busy, workspaceId } = snap;

  return (
    <>
      {snap.error && !snap.editorOpen ? (
        <div id="notes-cat-list-error" className="settings-result notes-cat-error">
          {snap.error}
        </div>
      ) : null}
      <div id="notes-cat-list">
        {snap.rows.map((row) => {
          const desc = row.description.trim();
          const confirming = snap.confirmDisableId === row.id;
          return (
            <div key={row.id} className="notes-cat-item" data-project={row.id}>
              <span className="notes-cat-item-main">
                <span className="notes-cat-item-title-row">
                  <span className="notes-cat-item-title">{row.title || row.id}</span>
                  {row.id === 'inbox' ? <span className="notes-cat-badge">Built-in</span> : null}
                </span>
                <span className={`notes-cat-item-desc${desc ? '' : ' is-empty'}`}>
                  {desc || 'No description'}
                </span>
              </span>
              <button
                type="button"
                className="btn-settings-ghost"
                disabled={busy}
                onClick={() => openNotesCategoryEditor(row.id)}
              >
                Edit
              </button>
              {row.id === 'inbox' ? null : confirming ? (
                <>
                  <button type="button" className="btn-settings-ghost" disabled={busy} onClick={cancelNotesCategoryDisable}>
                    Cancel
                  </button>
                  <button
                    type="button"
                    className="sediment-kb-delete-btn"
                    disabled={busy}
                    onClick={() => void disableNotesCategory(workspaceId, row.id)}
                  >
                    Confirm disable
                  </button>
                </>
              ) : (
                <button
                  type="button"
                  className="sediment-kb-delete-btn"
                  disabled={busy}
                  onClick={() => askNotesCategoryDisable(row.id)}
                >
                  Disable
                </button>
              )}
            </div>
          );
        })}
      </div>
      {snap.disabled.length ? (
        <details id="notes-projects-disabled" className="notes-disabled">
          <summary>Disabled ({snap.disabled.length})</summary>
          <p className="notes-cat-lead">
            Disabled projects are hidden. Their documents are kept, and the folder stays reserved.
          </p>
          {snap.disabled.map((row) => (
            <div key={row.id} className="notes-cat-item" data-project-disabled={row.id}>
              <span className="notes-cat-item-main">
                <span className="notes-cat-item-title">{row.title || row.id}</span>
              </span>
              <button
                type="button"
                className="btn-settings-ghost"
                disabled={busy}
                onClick={() => void enableNotesCategory(workspaceId, row.id)}
              >
                Enable
              </button>
            </div>
          ))}
        </details>
      ) : null}
    </>
  );
}
