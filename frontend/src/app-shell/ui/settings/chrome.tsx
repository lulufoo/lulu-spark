import { OverlayDismissButton } from '../../../shared/overlay-dismiss-button.tsx';
import { KbHidePatternsHost } from './kb-hide-patterns.tsx';
import { McpChannelToolsHost } from './mcp-channel-tools.tsx';
import { McpTicketsHost } from './mcp-tickets.tsx';
import { NotesSettingsPanel } from './notes-panel.tsx';
import {
  onSedimentKbAddSubmit,
  onSedimentKbManageAdd,
  SedimentKbManageList,
  SedimentKbRepoList,
} from './sediment-kb.tsx';

export function SettingsDialogChrome() {
  return (
    <div id="settings-dialog-box">
      <div id="settings-dialog-header">
        <span>Settings</span>
        <OverlayDismissButton id="btn-settings-close" title="Close" />
      </div>
      <div id="settings-dialog-body">
        <nav id="settings-nav">
          <button type="button" className="settings-nav-item active" data-panel="notes">Notes</button>
          <button type="button" className="settings-nav-item" data-panel="knowledge">Knowledge</button>
          <button type="button" className="settings-nav-item" data-panel="llm">Agent</button>
          <button type="button" className="settings-nav-item" data-panel="mcp">MCP</button>
        </nav>
        <div id="settings-panels">
          <NotesSettingsPanel />
          <div id="settings-panel-knowledge" className="settings-panel">
            <div className="settings-tabs" role="tablist">
              <button type="button" className="settings-tab active" data-tab="directory" role="tab">Directory</button>
              <button type="button" className="settings-tab" data-tab="categories" role="tab">Categories</button>
              <button type="button" className="settings-tab" data-tab="list" role="tab">List</button>
              <button type="button" className="settings-tab" data-tab="add" role="tab">Add</button>
              <button type="button" className="settings-tab" data-tab="hidden" role="tab">Hidden files</button>
            </div>
            <div
              id="settings-tab-knowledge-directory"
              className="settings-tab-panel active"
              data-tab="directory"
            >
              <div className="settings-field">
                <label htmlFor="knowledge-root-path">Knowledge directory</label>
                <input
                  id="knowledge-root-path"
                  type="text"
                  spellCheck={false}
                  autoComplete="off"
                  readOnly
                  className="settings-input-readonly"
                />
                <span className="settings-field-hint">
                  Fixed knowledge directory. Not configurable.
                </span>
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
            <div id="settings-tab-knowledge-list" className="settings-tab-panel" data-tab="list">
              <div id="repo-list-content">
                <SedimentKbRepoList />
              </div>
            </div>
            <div id="settings-tab-knowledge-add" className="settings-tab-panel" data-tab="add">
              <div className="settings-field">
                <label htmlFor="sediment-kb-add-name">Directory name</label>
                <input
                  id="sediment-kb-add-name"
                  type="text"
                  placeholder="topic-name"
                  autoComplete="off"
                  spellCheck={false}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') void onSedimentKbAddSubmit();
                  }}
                />
                <span className="settings-field-hint">
                  Creates a folder under the Knowledge directory.
                </span>
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
            <KbHidePatternsHost />
          </div>
          <div id="settings-panel-llm" className="settings-panel">
            <div className="settings-tabs" role="tablist">
              <button type="button" className="settings-tab active" data-tab="engine" role="tab">LLM</button>
            </div>
            <div id="settings-tab-llm-engine" className="settings-tab-panel active" data-tab="engine">
              <div className="settings-field">
                <label htmlFor="settings-llm-engine">LLM category</label>
                <select id="settings-llm-engine">
                  <option value="openai">OpenAI</option>
                  <option value="claude">Claude</option>
                  <option value="grok">Grok</option>
                  <option value="host">GLM</option>
                  <option value="kimi">Kimi</option>
                  <option value="qwen">Qwen</option>
                </select>
                <span className="settings-field-hint">
                  Platform is read-only. Model and Base URL can be edited.
                  Use the matching API key for the selected category.
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
                <label htmlFor="settings-llm-base-url">Base URL</label>
                <input
                  id="settings-llm-base-url"
                  type="text"
                  spellCheck={false}
                  autoComplete="off"
                  placeholder="Provider API base URL"
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
    </div>
  );
}
