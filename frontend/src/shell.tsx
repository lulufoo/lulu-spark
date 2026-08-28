// @ts-nocheck — chrome port; leftover islands still find these IDs.
import { openBindDialog } from './app-shell/bind-dialog.tsx';
import { ConvertDialog, openConvertDialog } from './app-shell/convert-dialog.tsx';
import { MoveDocDialog, openMoveDocDialog } from './app-shell/move-dialog.tsx';
import { openSettingsDialog } from './app-shell/settings-dialog.tsx';
import { DeleteDialog } from './notes/delete-dialog.tsx';
import { ReadLaterDialog } from './read-later/dialog.tsx';
import { CommentDeleteDialog } from './shared/comment-delete.tsx';

function closeMenuDropdowns() {
  document.getElementById('sync-menu-dropdown')?.classList.remove('open');
  document.getElementById('tools-menu-dropdown')?.classList.remove('open');
  document.getElementById('skills-menu-dropdown')?.classList.remove('open');
}

/** Desktop chrome as React nodes. Dialogs own their open state. */
export function Shell() {
  return (
    <>

      <header>
        <h1>
          <a id="btn-nav-home-title" href="#/home" className="header-home-link">LuLu Workbench</a>
          <a id="btn-nav-home" href="#/home" className="header-nav-back" hidden>← Home</a>
        </h1>
        <div id="gs-wb-wrap" className="gs-search-wrap" hidden>
          <input id="gs-wb-input" className="gs-search-input" type="text" placeholder="Search notes…" autoComplete="off" spellCheck={false} />
          <button id="gs-wb-rebuild-btn" className="gs-rebuild-btn" style={{ display: 'none' }} title="Rebuild Workbench index">↺</button>
          <div id="gs-wb-dropdown" className="gs-search-dropdown" style={{ display: 'none' }}></div>
        </div>
        <div id="gs-kb-wrap" className="gs-search-wrap" hidden>
          <input id="gs-kb-input" className="gs-search-input" type="text" placeholder="Search knowledge…" autoComplete="off" spellCheck={false} />
          <button id="gs-kb-rebuild-btn" className="gs-rebuild-btn" style={{ display: 'none' }} title="Rebuild knowledge index">↺</button>
          <div id="gs-kb-dropdown" className="gs-search-dropdown" style={{ display: 'none' }}></div>
        </div>
        <a href="https://github.com/lulufoo/lulu-workbench" target="_blank">GitHub ↗</a>
        <div id="sync-menu-wrap">
          <button id="btn-sync-menu">⇕ Sync</button>
          <div id="sync-menu-dropdown">
            <button id="btn-push-index">↑ Commit changes</button>
            <button id="btn-pull">↓ Update project</button>
            <button id="btn-local-refresh">⟳ Refresh local</button>
          </div>
        </div>
        <div id="tools-menu-wrap">
          <button id="btn-tools-menu">⛓ Tools</button>
          <div id="tools-menu-dropdown">
            <button
              id="btn-move-doc-header"
              type="button"
              onClick={() => {
                closeMenuDropdowns();
                openMoveDocDialog();
              }}
            >
              <span className="tools-menu-icon">↗</span>GitHub
            </button>
            <button
              id="btn-convert"
              type="button"
              onClick={() => {
                closeMenuDropdowns();
                openConvertDialog('base64');
              }}
            >
              <span className="tools-menu-icon">🔀</span>Convert
            </button>
            <button
              id="btn-bind"
              type="button"
              onClick={() => {
                closeMenuDropdowns();
                openBindDialog();
              }}
            >
              <span className="tools-menu-icon">📲</span>Bind device
            </button>
          </div>
        </div>
        <button
          id="btn-settings"
          type="button"
          title="Settings"
          onClick={() => {
            closeMenuDropdowns();
            void openSettingsDialog();
          }}
        >
          ⚙ Settings
        </button>
        <div id="skills-menu-wrap">
          <button id="btn-skills-menu">✦ SKILL</button>
          <div id="skills-menu-dropdown">
            <button id="btn-skill-workbench">✦ Lulu Workbench Skills</button>
          </div>
        </div>
      </header>

      <div id="home-view" style={{ display: 'none', overflow: 'hidden', height: 'calc(100vh - 52px)', boxSizing: 'border-box' }}></div>
      <div id="corpus-doc-view" style={{ display: 'none', height: 'calc(100vh - 52px)', boxSizing: 'border-box' }}></div>
      <div id="read-later-view" style={{ display: 'none', height: 'calc(100vh - 52px)', boxSizing: 'border-box' }}></div>
      <div id="todo-tasks-view" style={{ display: 'none', width: '100%', minWidth: 0, height: 'calc(100vh - 52px)', overflow: 'hidden', boxSizing: 'border-box' }}></div>

      <div className="layout">
        <aside id="sidebar">
          <div className="sidebar-inner">
            <div id="sidebar-channel-zone" className="sidebar-channel-zone"></div>
            <div id="sidebar-date-zone" className="sidebar-date-zone"></div>
          </div>
          <div id="sidebar-resizer" className="sidebar-resizer" role="separator" aria-orientation="vertical" aria-label="Resize sidebar" tabIndex="0"></div>
        </aside>
        <main id="main">
          <div id="status"><div style={{ color: '#8c959f', fontSize: '13px' }}>← Select a date to view documents</div></div>
          <div id="date-heading" style={{ display: 'none' }}></div>
          <div id="doc-list" className="doc-list"></div>
          <div id="note-outlet" hidden>
            <div id="note-outlet-message" className="note-outlet-message" hidden></div>
            <div id="md-panel" className="viewer-panel">
              <div id="md-header" className="viewer-header">
                <span id="md-panel-title" className="viewer-panel-title"></span>
                <div id="md-lang-bar" className="viewer-chrome-persisted" style={{ display: 'none', flexShrink: 0, alignItems: 'center', gap: '2px' }}>
                  <button className="md-header-btn" id="btn-lang-en">EN</button>
                  <button className="md-header-btn" id="btn-lang-zh">ZH</button>
                </div>
                <span id="md-file-size" className="viewer-chrome-persisted" style={{ fontSize: '10px', color: '#8c959f', flexShrink: 0 }}></span>
                <a id="md-github-link" className="viewer-chrome-persisted" href="#" target="_blank" style={{ fontSize: '12px', color: '#0969da', textDecoration: 'none', flexShrink: 0 }}>GitHub ↗</a>
                <button className="md-header-btn viewer-chrome-persisted" id="btn-goto-kb" style={{ display: 'none' }} title="Go to Knowledge">📚 Knowledge</button>
                <button className="md-header-btn viewer-chrome-persisted" id="btn-open-iterm" style={{ display: 'none' }} title="Open repo folder in iTerm">⌨️ Terminal</button>
                <button className="md-header-btn viewer-chrome-persisted" id="btn-copy-http" data-tip="">🌐</button>
                <button className="md-header-btn viewer-chrome-persisted" id="btn-copy-path" data-tip="">📁</button>
                <button className="md-header-btn viewer-chrome-persisted" id="btn-edit">✏️ Edit</button>
                <button className="md-header-btn viewer-chrome-persisted" id="btn-add-comment">💬 Comment</button>
                <button className="md-header-btn primary viewer-chrome-persisted" id="btn-save" style={{ display: 'none' }}>💾 Save</button>
                <button className="md-header-btn viewer-chrome-persisted" id="btn-cancel-edit" style={{ display: 'none' }}>Cancel</button>
                <button className="md-header-btn viewer-chrome-persisted" id="btn-panel-commit" style={{ display: 'none' }}>● Pending commit</button>
                <button id="md-close" className="md-header-btn viewer-close">✕ Close</button>
              </div>
              <div id="md-commit-bar" className="viewer-commit-bar viewer-chrome-persisted"></div>
              <div id="md-links-bar" className="viewer-chrome-persisted" style={{ display: 'none', padding: '8px 20px', borderBottom: '1px solid #d0d7de', display: 'flex', alignItems: 'center', flexWrap: 'wrap', gap: '4px 0' }}></div>
              <div id="md-tags-bar" className="viewer-chrome-persisted" style={{ display: 'none', padding: '8px 20px', borderBottom: '1px solid #d0d7de', flexWrap: 'wrap', alignItems: 'center', gap: '4px 0' }}></div>
              <div id="md-content-row" className="viewer-content-row">
                <div id="md-body" className="viewer-body"></div>
                <textarea id="md-edit-area" className="viewer-edit-area" style={{ display: 'none' }} spellCheck={false}></textarea>
                <div id="knowledge-panel" className="ks-collapsed viewer-chrome-persisted"></div>
                <div id="comment-float-nav" className="viewer-chrome-persisted"></div>
              </div>
            </div>
          </div>
          <div id="feed-view" style={{ display: 'none', padding: '20px', overflowY: 'auto', height: '100%', boxSizing: 'border-box' }}></div>
        </main>
      </div>

      {/* <!-- Convert tool dialog (Base64 + QR) --> */}
      <ConvertDialog />

      {/* <!-- Bind device dialog --> */}
      <div id="bind-dialog">
        <div id="bind-dialog-box">
          <div id="bind-dialog-header">
            <div id="bind-dialog-heading">
              <span id="bind-dialog-title">📲 Bind device</span>
              <p id="bind-dialog-lead">Pair this Mac with the Workbench Android app on the same local network.</p>
            </div>
            <button id="btn-bind-close">✕ Close</button>
          </div>
          <div id="bind-dialog-body">
            <div id="bind-qr-card">
              <div id="bind-preview"></div>
            </div>
            <div id="bind-dialog-meta">
              <div className="bind-meta-row">
                <span className="bind-meta-label">Status</span>
                <div id="bind-status" role="status" aria-live="polite"></div>
              </div>
              <div className="bind-meta-row">
                <span className="bind-meta-label">Code</span>
                <div id="bind-countdown"></div>
              </div>
              <p id="bind-host" hidden></p>
              <ol id="bind-steps">
                <li>Open the Workbench Android app.</li>
                <li>Scan this code on the same local network.</li>
                <li>Keep this window open until binding succeeds.</li>
              </ol>
            </div>
          </div>
          <div id="bind-dialog-footer">
            <p id="bind-footer-hint">A new code is available if this one expires.</p>
            <button id="btn-bind-refresh" hidden>New code</button>
          </div>
        </div>
      </div>

      {/* <!-- Workbench Commit dialog --> */}
      <div id="md-commit-dialog">
        <div id="md-commit-dialog-box">
          <div id="md-commit-dialog-title">
            <h3>● Commit changes</h3>
            <button id="md-btn-revert-all">Revert all changes</button>
          </div>
          <div id="md-commit-file-list"></div>
          <input id="md-commit-dialog-msg" type="text" placeholder="update: edit via viewer" />
          <div id="md-commit-dialog-actions">
            <span id="md-commit-dialog-result"></span>
            <button id="md-btn-commit-cancel" className="md-header-btn">Cancel</button>
            <button id="md-btn-commit-ok" className="md-header-btn primary">Commit</button>
          </div>
        </div>
      </div>

      {/* <!-- KB Commit dialog --> */}
      <div id="kb-commit-dialog">
        <div id="kb-commit-dialog-box">
          <div id="kb-commit-dialog-title">
            <h3>● Commit changes</h3>
            <button id="kb-btn-revert-all">Revert all changes</button>
          </div>
          <div id="kb-commit-file-list"></div>
          <input id="kb-commit-msg" type="text" placeholder="chore: update via viewer" />
          <div id="kb-commit-dialog-actions">
            <span id="kb-commit-result"></span>
            <button id="kb-btn-commit-cancel" className="md-header-btn">Cancel</button>
            <button id="kb-btn-commit-ok" className="md-header-btn primary">Commit</button>
          </div>
        </div>
      </div>

      {/* <!-- KB Comment dialog --> */}
      <div id="kb-comment-dialog">
        <div id="kb-comment-dialog-box">
          <div id="kb-comment-dialog-header">
            <h3 id="kb-comment-dialog-title">💬 Add comment</h3>
            <div id="kb-comment-dialog-tabs">
              <button className="comment-tab-btn active" data-tab="edit">Edit</button>
              <button className="comment-tab-btn" data-tab="preview">Preview</button>
            </div>
          </div>
          <div id="kb-comment-editor-box" className="comment-editor-box" onClick={() => { document.getElementById('kb-comment-dialog-content').focus() }}>
            <div id="kb-comment-dialog-content" className="comment-editor-content" contentEditable={true} spellCheck={false} autoCorrect="off" autoCapitalize="off" data-placeholder="Comment… (Ctrl/Cmd+Enter to save)"></div>
          </div>
          <div id="kb-comment-preview-pane" className="md-body" style={{ display: 'none' }}></div>
          <div id="kb-comment-dialog-actions">
            <button id="kb-btn-comment-cancel" className="md-header-btn">Cancel</button>
            <button id="kb-btn-comment-save" className="md-header-btn primary">Save</button>
          </div>
        </div>
      </div>

      {/* <!-- Floating highlight button --> */}
      <button id="highlight-add-btn" className="viewer-highlight-btn" style={{ display: 'none', position: 'fixed', zIndex: 9999 }}>Highlight</button>
      {/* <!-- KB Floating highlight button --> */}
      <button id="kb-highlight-add-btn" className="viewer-highlight-btn" style={{ display: 'none', position: 'fixed', zIndex: 9999 }}>Highlight</button>
      {/* <!-- Comment preview tooltip --> */}
      <div id="comment-preview-tip"></div>

      {/* <!-- Comment dialog --> */}
      <div id="comment-dialog">
        <div id="comment-dialog-box">
          <div id="comment-dialog-header">
            <h3 id="comment-dialog-title">💬 Add comment</h3>
            <div id="comment-dialog-tabs">
              <button className="comment-tab-btn active" data-tab="edit">Edit</button>
              <button className="comment-tab-btn" data-tab="preview">Preview</button>
            </div>
          </div>
          <div id="comment-editor-box" className="comment-editor-box" onClick={() => { document.getElementById('comment-dialog-content').focus() }}>
            <div id="comment-dialog-content" className="comment-editor-content" contentEditable={true} spellCheck={false} autoCorrect="off" autoCapitalize="off" data-placeholder="Comment… (Ctrl/Cmd+Enter to save)"></div>
          </div>
          <div id="comment-preview-pane" className="md-body" style={{ display: 'none' }}></div>
          <div id="comment-dialog-actions">
            <button id="btn-comment-cancel" className="md-header-btn">Cancel</button>
            <button id="btn-comment-save" className="md-header-btn primary">Save</button>
          </div>
        </div>
      </div>

      {/* <!-- Commit changes dialog --> */}
      <div id="commit-changes-dialog">
        <div id="commit-changes-dialog-box">
          <h3>↑ Commit changes</h3>
          <div id="commit-changes-file-list"></div>
          <input id="commit-changes-msg" type="text" placeholder="Commit message (empty: chore: update via viewer)" autoComplete="off" />
          <div id="commit-changes-dialog-actions">
            <span id="commit-changes-result"></span>
            <button id="btn-commit-changes-cancel" type="button">Cancel</button>
            <button id="btn-commit-changes-ok" type="button">Commit</button>
          </div>
        </div>
      </div>

      <div id="kb-diff-dialog">
        <div id="kb-diff-dialog-box">
          <div id="kb-diff-dialog-title-row">
            <h3 id="kb-diff-dialog-title">✎ Local changes</h3>
            <button id="btn-kb-diff-revert-all" type="button">Discard changes</button>
          </div>
          <div id="kb-diff-file-list"></div>
          <input id="kb-diff-msg" type="text" placeholder="Commit message (empty: chore: update via viewer)" autoComplete="off" />
          <div id="kb-diff-dialog-actions">
            <span id="kb-diff-result"></span>
            <button id="btn-kb-diff-cancel" type="button">Cancel</button>
            <button id="btn-kb-diff-ok" type="button">Commit</button>
          </div>
        </div>
      </div>

      {/* <!-- GitHub ops dialog (move + delete) --> */}
      <MoveDocDialog />

      {/* <!-- Comment note delete confirm --> */}
      <CommentDeleteDialog />

      {/* <!-- Delete confirm dialog --> */}
      <DeleteDialog />

      {/* <!-- Move project dialog --> */}
      <div id="move-project-dialog">
        <div id="move-project-backdrop"></div>
        <div id="move-project-dialog-box">
          <h3>↷ Switch project</h3>
          <div id="move-project-result" style={{ fontSize: '12px', color: '#57606a', marginBottom: '8px' }}></div>
          <div id="move-project-list" style={{ display: 'flex', flexDirection: 'column', gap: '4px', maxHeight: '300px', overflowY: 'auto' }}></div>
          <div id="move-project-dialog-actions">
            <button id="btn-move-project-close">Cancel</button>
          </div>
        </div>
      </div>

      {/* <!-- Settle dialog --> */}
      <div id="settle-dialog">
        <div id="settle-dialog-box">
          <h3>⬆ Promote to knowledge repo</h3>
          <div className="settle-field">
            <label>Target repo</label>
            <span id="settle-repo-display"></span>
          </div>
          <div className="settle-field">
            <label>Target folder (doc-theme)</label>
            <select id="settle-theme-select"></select>
            <input id="settle-theme-input" type="text" placeholder="＋ New folder…" autoComplete="off" style={{ display: 'none' }} />
          </div>
          <div className="settle-field">
            <label>Filename</label>
            <div className="settle-filename-row">
              <span className="settle-filename-ts" id="settle-filename-ts"></span><span className="settle-filename-dash">-</span><input id="settle-slug" type="text" placeholder="Enter name" autoComplete="off" spellCheck={false} /><span className="settle-filename-ext">.md</span>
            </div>
            <div id="settle-file-warn"></div>
          </div>
          <div className="settle-field">
            <label>Body</label>
            <textarea id="settle-content" rows="10"></textarea>
          </div>
          <div id="settle-result"></div>
          <div id="settle-dialog-actions">
            <button id="btn-settle-cancel" className="md-header-btn">Cancel</button>
            <button id="btn-settle-submit" className="md-header-btn primary">Push</button>
          </div>
        </div>
      </div>

      {/* <!-- Settings dialog --> */}
      <div id="settings-dialog">
        <div id="settings-dialog-box">
          <div id="settings-dialog-header">
            <span>Settings</span>
            <button id="btn-settings-close" type="button" title="Close">✕</button>
          </div>
          <div id="settings-dialog-body">
            <nav id="settings-nav">
              <button type="button" className="settings-nav-item active" data-panel="directories">Notes</button>
              <button type="button" className="settings-nav-item" data-panel="knowledge">Knowledge</button>
              <button type="button" className="settings-nav-item" data-panel="llm">Assistant</button>
              <button type="button" className="settings-nav-item" data-panel="github">Sync</button>
              <button type="button" className="settings-nav-item" data-panel="mcp">MCP</button>
            </nav>
            <div id="settings-panels">
              <div id="settings-panel-directories" className="settings-panel active">
                <div className="settings-tabs" role="tablist">
                  <button type="button" className="settings-tab active" data-tab="directory" role="tab">Directory</button>
                  <button type="button" className="settings-tab" data-tab="connection" role="tab">Connection</button>
                </div>
                <div id="settings-tab-notes-directory" className="settings-tab-panel active" data-tab="directory">
                  <div className="settings-field">
                    <input id="settings-archive-root" type="text" spellCheck={false} autoComplete="off" aria-label="Directory" />
                    <span className="settings-field-hint">Local DDM archive repo; if already a git repo, GitHub profile is inferred from origin on save (see Sync).</span>
                  </div>
                  <div id="settings-result-directories" className="settings-result"></div>
                  <div className="settings-panel-actions">
                    <button type="button" id="btn-settings-save-directories" className="btn-settings-save">Save</button>
                  </div>
                </div>
                <div id="settings-tab-notes-connection" className="settings-tab-panel" data-tab="connection">
                  <div id="notes-connect-add" className="settings-field">
                    <label htmlFor="notes-connect-url">GitHub repository URL</label>
                    <div className="settings-connect-row">
                      <input id="notes-connect-url" type="text" placeholder="owner/repo or GitHub URL" autoComplete="off" spellCheck={false} />
                      <button id="btn-notes-connect-add" type="button" className="btn-settings-save">Add</button>
                    </div>
                  </div>
                  <div id="notes-connect-error" className="settings-result"></div>
                  <div id="notes-connect-item"></div>
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
                <div id="settings-tab-knowledge-directory" className="settings-tab-panel active" data-tab="directory">
                  <div className="settings-field">
                    <label htmlFor="sediment-kb-corpus-path">Knowledge corpus directory (knowledge_corpus_root)</label>
                    <input id="sediment-kb-corpus-path" type="text" spellCheck={false} autoComplete="off" />
                    <span className="settings-field-hint">Clone root for topic knowledge repos</span>
                  </div>
                  <div id="sediment-kb-corpus-error" className="settings-result"></div>
                  <div className="settings-panel-actions">
                    <button id="btn-sediment-kb-corpus-save" type="button" className="btn-settings-save">Save</button>
                  </div>
                </div>
                <div id="settings-tab-knowledge-list" className="settings-tab-panel" data-tab="list">
                  <div id="repo-list-title-group" className="settings-knowledge-list-head">
                    <button id="btn-repo-list-refresh" type="button" className="btn-settings-save" title="Refresh">Refresh</button>
                  </div>
                  <div id="repo-list-content"></div>
                </div>
                <div id="settings-tab-knowledge-categories" className="settings-tab-panel" data-tab="categories">
                  <div id="sediment-kb-manage-list"></div>
                  <div id="sediment-kb-manage-add-row" className="settings-connect-row">
                    <input id="sediment-kb-manage-new-name" type="text" placeholder="New category name" autoComplete="off" spellCheck={false} />
                    <button id="btn-sediment-kb-manage-add" type="button" className="btn-settings-save">Add category</button>
                  </div>
                  <div id="sediment-kb-manage-error" className="settings-result"></div>
                </div>
                <div id="settings-tab-knowledge-add" className="settings-tab-panel" data-tab="add">
                  <div className="settings-field">
                    <label htmlFor="sediment-kb-add-url">GitHub repository URL</label>
                    <input id="sediment-kb-add-url" type="text" placeholder="owner/repo or GitHub URL" autoComplete="off" spellCheck={false} />
                  </div>
                  <div className="settings-field">
                    <label htmlFor="sediment-kb-add-category">Category (optional)</label>
                    <select id="sediment-kb-add-category"></select>
                  </div>
                  <div className="settings-field">
                    <label htmlFor="sediment-kb-add-description">Description (optional)</label>
                    <textarea id="sediment-kb-add-description" rows="3" placeholder="Repository description" autoComplete="off" spellCheck={false}></textarea>
                  </div>
                  <div id="sediment-kb-add-error" className="settings-result"></div>
                  <div className="settings-panel-actions">
                    <button id="btn-sediment-kb-add-submit" type="button" className="btn-settings-save">Add</button>
                  </div>
                </div>
                <div id="settings-tab-knowledge-hidden" className="settings-tab-panel" data-tab="hidden">
                  <div className="settings-field">
                    <label htmlFor="settings-kb-hide-pattern">Hidden filename regex (kb_hide_pattern)</label>
                    <input id="settings-kb-hide-pattern" type="text" spellCheck={false} autoComplete="off" placeholder="" />
                    <span className="settings-field-hint">Files matching entry.name are hidden in the tree; leave empty for no filter. Example: <code>\\.xxx$</code></span>
                  </div>
                  <div id="settings-result-knowledge" className="settings-result"></div>
                  <div className="settings-panel-actions">
                    <button type="button" id="btn-settings-save-knowledge" className="btn-settings-save">Save</button>
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
                      <option value="host">Agent Loop / GLM</option>
                    </select>
                    <span className="settings-field-hint">Agent Loop / GLM is the only supported assistant engine. Preset fields below are read-only except Model.</span>
                  </div>
                  <div className="settings-field">
                    <label htmlFor="settings-llm-platform">Platform (read-only)</label>
                    <input id="settings-llm-platform" type="text" spellCheck={false} autoComplete="off" readOnly className="settings-input-readonly" />
                  </div>
                  <div className="settings-field">
                    <label htmlFor="settings-llm-base-url">Base URL (read-only)</label>
                    <input id="settings-llm-base-url" type="text" spellCheck={false} autoComplete="off" readOnly className="settings-input-readonly" />
                  </div>
                  <div className="settings-field">
                    <label htmlFor="settings-llm-model">Model (model)</label>
                    <input id="settings-llm-model" type="text" spellCheck={false} autoComplete="off" placeholder="Agent model id" />
                  </div>
                  <div className="settings-field">
                    <label htmlFor="settings-llm-api-key">Credential</label>
                    <input id="settings-llm-api-key" type="password" autoComplete="off" />
                    <span id="settings-llm-key-hint" className="settings-field-hint"></span>
                  </div>
                  <div id="settings-result-llm" className="settings-result"></div>
                  <div className="settings-panel-actions">
                    <button type="button" id="btn-settings-save-llm" className="btn-settings-save">Save</button>
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
                    <input id="settings-github-user-url" type="text" spellCheck={false} autoComplete="off" placeholder="https://github.com/lulufoo" />
                    <span id="settings-github-user-hint" className="settings-field-hint">Inferred from the Notes directory origin when possible; used for Viewer remote links.</span>
                  </div>
                  <div className="settings-field">
                    <label htmlFor="settings-github-token">GitHub token (sync key)</label>
                    <input id="settings-github-token" type="password" autoComplete="off" />
                    <span id="settings-token-hint" className="settings-field-hint">Used for Notes and Knowledge GitHub sync.</span>
                  </div>
                  <div id="settings-result-github" className="settings-result"></div>
                  <div className="settings-panel-actions">
                    <button type="button" id="btn-settings-save-github" className="btn-settings-save">Save</button>
                  </div>
                </div>
              </div>
              <div id="settings-panel-mcp" className="settings-panel">
                <div className="settings-field">
                  <label htmlFor="settings-mcp-server-block">Cursor IDE server block</label>
                  <textarea id="settings-mcp-server-block" rows="8" spellCheck={false} autoComplete="off" readOnly></textarea>
                  <span className="settings-field-hint">Generate copies a full server block for you to paste into Cursor. Host does not write that file.</span>
                </div>
                <div className="settings-panel-actions">
                  <button type="button" id="btn-settings-mcp-generate" className="btn-settings-save">Generate & copy</button>
                  <button type="button" id="btn-settings-mcp-rotate" className="btn-settings-save">Rotate cursor_ide</button>
                </div>
                <div className="settings-field">
                  <label htmlFor="settings-mcp-revoke-slot">Revoke current ticket</label>
                  <div className="settings-connect-row">
                    <select id="settings-mcp-revoke-slot" aria-label="Slot to revoke">
                      <option value="cursor_ide">cursor_ide</option>
                      <option value="workbench">workbench</option>
                    </select>
                    <button type="button" id="btn-settings-mcp-revoke" className="btn-settings-save">Revoke</button>
                  </div>
                </div>
                <div id="settings-result-mcp" className="settings-result"></div>
              </div>
            </div>
          </div>
          <div id="settings-dialog-footer">
            <button type="button" id="btn-settings-cancel">Close</button>
          </div>
        </div>
      </div>

      {/* <!-- Plan Task dialog --> */}
      <div id="todo-task-dialog">
        <div id="todo-task-dialog-box">
          <div id="todo-task-dialog-header">
            <h3 id="todo-task-dialog-title"></h3>
          </div>
          <div id="todo-task-dialog-body"></div>
          <p id="todo-task-dialog-error" hidden></p>
          <div id="todo-task-dialog-actions">
            <button type="button" id="todo-task-dialog-cancel" className="md-header-btn">Cancel</button>
            <button type="button" id="todo-task-dialog-primary" className="md-header-btn primary">OK</button>
          </div>
        </div>
      </div>

      {/* <!-- Read Later dialog --> */}
      <ReadLaterDialog />

      {/* <!-- Skills dialog --> */}
      <div id="skills-dialog">
        <div id="skills-dialog-box">
          <div id="skills-dialog-header">
            <h3 id="skills-dialog-title"></h3>
            <button id="btn-skills-dialog-close" className="md-header-btn">✕ Close</button>
          </div>
          <div id="skills-dialog-body"></div>
        </div>
      </div>

    </>
  );
}
