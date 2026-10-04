import { SparkConnectionHost } from './spark-connection.tsx';

export function SparkSettingsPanel() {
  return (
    <div id="settings-panel-spark" className="settings-panel active">
      <div className="settings-tabs" role="tablist">
        <button type="button" className="settings-tab active" data-tab="directory" role="tab">
          Directory
        </button>
        <button type="button" className="settings-tab" data-tab="connection" role="tab">
          Connection
        </button>
      </div>
      <div id="settings-tab-spark-directory" className="settings-tab-panel active" data-tab="directory">
        <div className="settings-field">
          <input
            id="settings-spark-root"
            type="text"
            spellCheck={false}
            autoComplete="off"
            aria-label="Directory"
          />
          <span className="settings-field-hint">
            Local data store (notes, todos, Knowledge registry). If it is a git repo, GitHub
            profile is inferred from origin on save (see Sync).
          </span>
        </div>
        <div id="settings-result-spark" className="settings-result" />
        <div className="settings-panel-actions">
          <button type="button" id="btn-settings-save-spark" className="btn-settings-save">
            Save
          </button>
        </div>
      </div>
      <div id="settings-tab-spark-connection" className="settings-tab-panel" data-tab="connection">
        <div id="spark-connect-add" className="settings-field">
          <label htmlFor="spark-connect-url">GitHub repository URL</label>
          <div className="settings-connect-row">
            <input
              id="spark-connect-url"
              type="text"
              placeholder="owner/repo or GitHub URL"
              autoComplete="off"
              spellCheck={false}
            />
            <button id="btn-spark-connect-add" type="button" className="btn-settings-save">
              Add
            </button>
          </div>
        </div>
        <div id="spark-connect-error" className="settings-result" />
        <div id="spark-connect-item">
          <SparkConnectionHost />
        </div>
      </div>
    </div>
  );
}
