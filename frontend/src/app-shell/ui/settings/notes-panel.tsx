import { NotesWorkspacesHost } from './notes-workspaces.tsx';

/** The Workspaces panel manages workspaces only; projects live on each workspace's own page. */
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
      </div>
    </div>
  );
}
