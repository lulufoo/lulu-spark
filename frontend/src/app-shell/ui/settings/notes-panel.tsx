import { NotesCategoriesAddHost } from './notes-categories-add.tsx';
import { NotesWorkspacesHost } from './notes-workspaces.tsx';
import { NotesCategoriesEditHost } from './notes-categories-edit.tsx';

/**
 * The Workspaces panel manages workspaces only. Project editing stays reachable in the
 * collapsed section below until each workspace gets its own settings page.
 */
export function NotesSettingsPanel() {
  return (
    <div id="settings-panel-notes" className="settings-panel active">
      <div className="notes-workspaces-panel">
        <div className="settings-field">
          <label htmlFor="workspaces-root-path">Workspaces directory</label>
          <input
            id="workspaces-root-path"
            type="text"
            spellCheck={false}
            autoComplete="off"
            readOnly
            className="settings-input-readonly"
          />
          <span className="settings-field-hint">
            Fixed workspaces root. Not configurable.
          </span>
        </div>
        <NotesWorkspacesHost />
        <details className="notes-disabled" id="notes-projects-interim">
          <summary>Projects of the current workspace</summary>
          <div id="settings-tab-notes-add">
            <NotesCategoriesAddHost />
          </div>
          <div id="settings-tab-notes-edit">
            <NotesCategoriesEditHost />
          </div>
        </details>
      </div>
    </div>
  );
}
