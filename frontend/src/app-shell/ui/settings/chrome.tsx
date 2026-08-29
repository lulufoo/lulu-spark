import { McpChannelToolsHost } from './mcp-channel-tools.tsx';
import { McpTicketsHost } from './mcp-tickets.tsx';
import { WorkbenchConnectionHost } from './workbench-connection.tsx';
import {
  onRepoListRefresh,
  onSedimentKbAddSubmit,
  onSedimentKbManageAdd,
  SedimentKbAddCategorySelect,
  SedimentKbManageList,
  SedimentKbRepoList,
} from './sediment-kb.tsx';

export function SettingsDialogChrome() {
  return (
    <div id="settings-dialog-box">
      <div id="settings-dialog-header">
        <span>Settings</span>
        <button id="btn-settings-close" type="button" title="Close">
          ✕
        </button>
      </div>
      <div id="settings-dialog-body">
        <nav id="settings-nav">
          <button type="button" className="settings-nav-item active" data-panel="workbench">Workbench</button>
          <button type="button" className="settings-nav-item" data-panel="knowledge">Knowledge</button>
          <button type="button" className="settings-nav-item" data-panel="llm">Assistant</button>
          <button type="button" className="settings-nav-item" data-panel="github">Sync</button>
          <button type="button" className="settings-nav-item" data-panel="mcp">MCP</button>
        </nav>
        <div id="settings-panels">
          <div id="settings-panel-workbench" className="settings-panel active">
            <div className="settings-tabs" role="tablist">
              <button type="button" className="settings-tab active" data-tab="directory" role="tab">Directory</button>
              <button type="button" className="settings-tab" data-tab="connection" role="tab">Connection</button>
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
          <div id="settings-panel-knowledge" className="settings-panel">
            <div className="settings-tabs" role="tablist">
              <button type="button" className="settings-tab active" data-tab="directory" role="tab">Directory</button>
              <button type="button" className="settings-tab" data-tab="list" role="tab">List</button>
              <button type="button" className="settings-tab" data-tab="categories" role="tab">Categories</button>
              <button type="button" className="settings-tab" data-tab="add" role="tab">Add</button>
              <button type="button" className="settings-tab" data-tab="hidden" role="tab">Hidden files</button>
            </div>
            <div
              id="settings-tab-knowledge-directory"
              className="settings-tab-panel active"
              data-tab="directory"
            >
              <div className="settings-field">
                <label htmlFor="knowledge-root-path">
                  Knowledge directory (knowledge_root)
                </label>
                <input id="knowledge-root-path" type="text" spellCheck={false} autoComplete="off" />
                <span className="settings-field-hint">Clone root for topic knowledge repos</span>
              </div>
              <div id="knowledge-root-error" className="settings-result" />
              <div className="settings-panel-actions">
                <button id="btn-knowledge-root-save" type="button" className="btn-settings-save">
                  Save
                </button>
              </div>
            </div>
            <div id="settings-tab-knowledge-list" className="settings-tab-panel" data-tab="list">
              <div id="repo-list-title-group" className="settings-knowledge-list-head">
                <button
                  id="btn-repo-list-refresh"
                  type="button"
                  className="btn-settings-save"
                  title="Refresh"
                  onClick={() => onRepoListRefresh()}
                >
                  Refresh
                </button>
              </div>
              <div id="repo-list-content">
                <SedimentKbRepoList />
              </div>
            </div>
            <div id="settings-tab-knowledge-categories" className="settings-tab-panel" data-tab="categories">
              <div id="sediment-kb-manage-list">
                <SedimentKbManageList />
              </div>
              <div id="sediment-kb-manage-add-row" className="settings-connect-row">
                <input
                  id="sediment-kb-manage-new-name"
                  type="text"
                  placeholder="New category name"
                  autoComplete="off"
                  spellCheck={false}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') void onSedimentKbManageAdd();
                  }}
                />
                <button
                  id="btn-sediment-kb-manage-add"
                  type="button"
                  className="btn-settings-save"
                  onClick={() => void onSedimentKbManageAdd()}
                >
                  Add category
                </button>
              </div>
              <div id="sediment-kb-manage-error" className="settings-result" />
            </div>
            <div id="settings-tab-knowledge-add" className="settings-tab-panel" data-tab="add">
              <div className="settings-field">
                <label htmlFor="sediment-kb-add-url">GitHub repository URL</label>
                <input
                  id="sediment-kb-add-url"
                  type="text"
                  placeholder="owner/repo or GitHub URL"
                  autoComplete="off"
                  spellCheck={false}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') void onSedimentKbAddSubmit();
                  }}
                />
              </div>
              <div className="settings-field">
                <label htmlFor="sediment-kb-add-category">Category (optional)</label>
                <SedimentKbAddCategorySelect />
              </div>
              <div className="settings-field">
                <label htmlFor="sediment-kb-add-description">Description (optional)</label>
                <textarea id="sediment-kb-add-description" rows={3} placeholder="Repository description" autoComplete="off" spellCheck={false} />
              </div>
              <div id="sediment-kb-add-error" className="settings-result" />
              <div className="settings-panel-actions">
                <button
                  id="btn-sediment-kb-add-submit"
                  type="button"
                  className="btn-settings-save"
                  onClick={() => void onSedimentKbAddSubmit()}
                >
                  Add
                </button>
              </div>
            </div>
            <div id="settings-tab-knowledge-hidden" className="settings-tab-panel" data-tab="hidden">
              <div className="settings-field">
                <label htmlFor="settings-kb-hide-pattern">Hidden filename regex (kb_hide_pattern)</label>
                <input
                  id="settings-kb-hide-pattern"
                  type="text"
                  spellCheck={false}
                  autoComplete="off"
                  placeholder=""
                />
                <span className="settings-field-hint">
                  Files matching entry.name are hidden in the tree; leave empty for no filter. Example:{' '}
                  <code>{'\\.xxx$'}</code>
                </span>
              </div>
              <div id="settings-result-knowledge" className="settings-result" />
              <div className="settings-panel-actions">
                <button type="button" id="btn-settings-save-knowledge" className="btn-settings-save">
                  Save
                </button>
              </div>
            </div>
          </div>
          <div id="settings-panel-llm" className="settings-panel">
            <div className="settings-tabs" role="tablist">
              <button type="button" className="settings-tab active" data-tab="engine" role="tab">Engine</button>
            </div>
            <div id="settings-tab-llm-engine" className="settings-tab-panel active" data-tab="engine">
              <div className="settings-field">
                <label htmlFor="settings-llm-engine">Engine category</label>
                <select id="settings-llm-engine">
                  <option value="host">GLM</option>
                </select>
                <span className="settings-field-hint">
                  GLM is the only supported assistant engine. Preset fields below are read-only
                  except Model.
                </span>
              </div>
              <div className="settings-field">
                <label htmlFor="settings-llm-platform">Platform (read-only)</label>
                <input
                  id="settings-llm-platform"
                  type="text"
                  spellCheck={false}
                  autoComplete="off"
                  readOnly
                  className="settings-input-readonly"
                />
              </div>
              <div className="settings-field">
                <label htmlFor="settings-llm-base-url">Base URL (read-only)</label>
                <input
                  id="settings-llm-base-url"
                  type="text"
                  spellCheck={false}
                  autoComplete="off"
                  readOnly
                  className="settings-input-readonly"
                />
              </div>
              <div className="settings-field">
                <label htmlFor="settings-llm-model">Model (model)</label>
                <input
                  id="settings-llm-model"
                  type="text"
                  spellCheck={false}
                  autoComplete="off"
                  placeholder="Agent model id"
                />
              </div>
              <div className="settings-field">
                <label htmlFor="settings-llm-api-key">Credential</label>
                <input id="settings-llm-api-key" type="password" autoComplete="off" />
                <span id="settings-llm-key-hint" className="settings-field-hint" />
              </div>
              <div id="settings-result-llm" className="settings-result" />
              <div className="settings-panel-actions">
                <button type="button" id="btn-settings-save-llm" className="btn-settings-save">
                  Save
                </button>
              </div>
            </div>
          </div>
          <div id="settings-panel-github" className="settings-panel">
            <div className="settings-tabs" role="tablist">
              <button type="button" className="settings-tab active" data-tab="account" role="tab">Account</button>
            </div>
            <div id="settings-tab-github-account" className="settings-tab-panel active" data-tab="account">
              <div className="settings-field">
                <label htmlFor="settings-github-user-url">GitHub profile (github_user_url)</label>
                <input
                  id="settings-github-user-url"
                  type="text"
                  spellCheck={false}
                  autoComplete="off"
                  placeholder="https://github.com/lulufoo"
                />
                <span id="settings-github-user-hint" className="settings-field-hint">
                  Inferred from the Workbench directory origin when possible; used for Viewer remote links.
                </span>
              </div>
              <div className="settings-field">
                <label htmlFor="settings-github-token">GitHub token (sync key)</label>
                <input id="settings-github-token" type="password" autoComplete="off" />
                <span id="settings-token-hint" className="settings-field-hint">
                  Used for Workbench and Knowledge GitHub sync.
                </span>
              </div>
              <div id="settings-result-github" className="settings-result" />
              <div className="settings-panel-actions">
                <button type="button" id="btn-settings-save-github" className="btn-settings-save">
                  Save
                </button>
              </div>
            </div>
          </div>
          <div id="settings-panel-mcp" className="settings-panel">
            <div className="settings-tabs" role="tablist">
              <button type="button" className="settings-tab active" data-tab="tickets" role="tab">Tickets</button>
              <button type="button" className="settings-tab" data-tab="tools" role="tab">Tools</button>
            </div>
            <div id="settings-tab-mcp-tickets" className="settings-tab-panel active" data-tab="tickets">
              <McpTicketsHost />
            </div>
            <div id="settings-tab-mcp-tools" className="settings-tab-panel" data-tab="tools">
              <McpChannelToolsHost />
            </div>
          </div>
        </div>
      </div>
      <div id="settings-dialog-footer">
        <button type="button" id="btn-settings-cancel">
          Close
        </button>
      </div>
    </div>
  );
}
