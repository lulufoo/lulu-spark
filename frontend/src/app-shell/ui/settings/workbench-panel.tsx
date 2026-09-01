import { WorkbenchConnectionHost } from './workbench-connection.tsx';

export function WorkbenchSettingsPanel() {
  return (
    <div id="settings-panel-workbench" className="settings-panel active">
      <div className="settings-tabs" role="tablist">
        <button type="button" className="settings-tab active" data-tab="directory" role="tab">
          Directory
        </button>
        <button type="button" className="settings-tab" data-tab="connection" role="tab">
          Connection
        </button>
      </div>
      <div id="settings-tab-workbench-directory" className="settings-tab-panel active" data-tab="directory">
        <div className="settings-field">
          <input
            id="settings-workbench-root"
            type="text"
            spellCheck={false}
            autoComplete="off"
            aria-label="Directory"
          />
          <span className="settings-field-hint">
            Local Workbench data store (notes, todos, Knowledge registry). If it is a git repo, GitHub
            profile is inferred from origin on save (see Sync).
          </span>
        </div>
        <div id="settings-result-workbench" className="settings-result" />
        <div className="settings-panel-actions">
          <button type="button" id="btn-settings-save-workbench" className="btn-settings-save">
            Save
          </button>
        </div>
      </div>
      <div id="settings-tab-workbench-connection" className="settings-tab-panel" data-tab="connection">
        <div id="workbench-connect-add" className="settings-field">
          <label htmlFor="workbench-connect-url">GitHub repository URL</label>
          <div className="settings-connect-row">
            <input
              id="workbench-connect-url"
              type="text"
              placeholder="owner/repo or GitHub URL"
              autoComplete="off"
              spellCheck={false}
            />
            <button id="btn-workbench-connect-add" type="button" className="btn-settings-save">
              Add
            </button>
          </div>
        </div>
        <div id="workbench-connect-error" className="settings-result" />
        <div id="workbench-connect-item">
          <WorkbenchConnectionHost />
        </div>
      </div>
    </div>
  );
}
