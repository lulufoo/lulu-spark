import { useEffect, type CSSProperties } from 'react';
import { applySearchNavChrome } from './app-shell/nav-chrome.ts';
import { getHomeEntryShell } from './home-entry-shell/access.ts';
import { getActivePath } from './corpus/corpus-path.ts';
import { useHostState } from './host/state.ts';
import { closeModal, enterEditMode, exitEditMode, saveDoc, switchLang } from './notes/viewer.ts';
import { openCommitDialog } from './notes/viewer/commit.tsx';

function slotStyle(visible: boolean, shown: CSSProperties): CSSProperties {
  return visible ? shown : { ...shown, display: 'none' };
}

function copyHttpLink() {
  const btn = document.getElementById('btn-copy-http');
  const url = btn?.dataset.url || '';
  if (!url || !btn) return;
  void navigator.clipboard.writeText(url).then(() => {
    btn.textContent = '✓';
    setTimeout(() => {
      btn.textContent = '🌐';
    }, 1200);
  });
}

function copyLocalPath(
  entry: { translations?: { zh?: string }; common_path?: string } | null,
  lang: string | null,
  layer: string,
  workbenchRoot: string,
) {
  if (!entry) return;
  const activePath = getActivePath(entry, lang || '', layer);
  const relPath = `${layer}/${activePath}`;
  const fullPath = workbenchRoot ? `${workbenchRoot}/${relPath}` : relPath;
  const btn = document.getElementById('btn-copy-path');
  if (!btn) return;
  void navigator.clipboard.writeText(fullPath).then(() => {
    btn.textContent = '✓';
    setTimeout(() => {
      btn.textContent = '📂';
    }, 1200);
  });
}

/** Five hash pages. Always mounted so leftover island code can keep writing into these IDs. */
export function ShellPages({ routeName }: { routeName: string }) {
  const host = useHostState();
  const homeOn = routeName === 'home' || routeName === 'read-later';
  const corpusOn = routeName === 'corpus-doc';
  const todoOn = routeName === 'todo-tasks';
  const notesOn = routeName === 'workbench';

  useEffect(() => {
    applySearchNavChrome(routeName);
  }, [routeName]);

  useEffect(() => {
    const list = document.getElementById('doc-list');
    if (!list) return undefined;
    const onScroll = () => {
      if (host.ui.activeDate) {
        sessionStorage.setItem('cta_scroll_' + host.ui.activeDate, String(list.scrollTop));
      }
    };
    list.addEventListener('scroll', onScroll, { passive: true });
    return () => list.removeEventListener('scroll', onScroll);
  }, [host.ui.activeDate]);

  return (
    <>
      <div
        id="home-view"
        style={slotStyle(homeOn, {
          overflow: 'hidden',
          height: 'calc(100vh - 52px)',
          boxSizing: 'border-box',
        })}
      />
      <div
        id="corpus-doc-view"
        style={slotStyle(corpusOn, {
          height: 'calc(100vh - 52px)',
          boxSizing: 'border-box',
        })}
      />
      <div
        id="read-later-view"
        style={slotStyle(false, {
          height: 'calc(100vh - 52px)',
          boxSizing: 'border-box',
        })}
      />
      <div
        id="todo-tasks-view"
        style={slotStyle(todoOn, {
          width: '100%',
          minWidth: 0,
          height: 'calc(100vh - 52px)',
          overflow: 'hidden',
          boxSizing: 'border-box',
        })}
      />

      <div
        className="layout"
        data-active-date={host.ui.activeDate || ''}
        style={notesOn ? undefined : { display: 'none' }}
      >
        <aside id="sidebar">
          <div className="sidebar-inner">
            <div id="sidebar-channel-zone" className="sidebar-channel-zone"></div>
            <div id="sidebar-date-zone" className="sidebar-date-zone"></div>
          </div>
          <div
            id="sidebar-resizer"
            className="sidebar-resizer"
            role="separator"
            aria-orientation="vertical"
            aria-label="Resize sidebar"
            tabIndex={0}
          ></div>
        </aside>
        <main id="main">
          <div id="status">
            <div style={{ color: '#8c959f', fontSize: '13px' }}>← Select a date to view documents</div>
          </div>
          <div id="date-heading" style={{ display: 'none' }}></div>
          <div id="doc-list" className="doc-list"></div>
          <div id="note-outlet" hidden>
            <div id="note-outlet-message" className="note-outlet-message" hidden></div>
            <div id="md-panel" className="viewer-panel">
              <div id="md-header" className="viewer-header">
                <span id="md-panel-title" className="viewer-panel-title"></span>
                <div
                  id="md-lang-bar"
                  className="viewer-chrome-persisted"
                  style={{ display: 'none', flexShrink: 0, alignItems: 'center', gap: '2px' }}
                >
                  <button className="md-header-btn" id="btn-lang-en" type="button" onClick={() => switchLang('en')}>
                    EN
                  </button>
                  <button className="md-header-btn" id="btn-lang-zh" type="button" onClick={() => switchLang('zh')}>
                    ZH
                  </button>
                </div>
                <span
                  id="md-file-size"
                  className="viewer-chrome-persisted"
                  style={{ fontSize: '10px', color: '#8c959f', flexShrink: 0 }}
                ></span>
                <a
                  id="md-github-link"
                  className="viewer-chrome-persisted"
                  href="#"
                  target="_blank"
                  style={{ fontSize: '12px', color: '#0969da', textDecoration: 'none', flexShrink: 0 }}
                >
                  GitHub ↗
                </a>
                <button
                  className="md-header-btn viewer-chrome-persisted"
                  id="btn-goto-kb"
                  style={{ display: 'none' }}
                  title="Go to Knowledge"
                >
                  📚 Knowledge
                </button>
                <button
                  className="md-header-btn viewer-chrome-persisted"
                  id="btn-open-iterm"
                  style={{ display: 'none' }}
                  title="Open repo folder in iTerm"
                >
                  ⌨️ Terminal
                </button>
                <button className="md-header-btn viewer-chrome-persisted" id="btn-copy-http" type="button" data-tip="" onClick={copyHttpLink}>
                  🌐
                </button>
                <button
                  className="md-header-btn viewer-chrome-persisted"
                  id="btn-copy-path"
                  type="button"
                  data-tip=""
                  onClick={() =>
                    copyLocalPath(
                      host.viewer.entry,
                      host.viewer.lang,
                      host.viewer.layer,
                      host.ui.workbenchKnowledgeRoot,
                    )
                  }
                >
                  📁
                </button>
                <button
                  className="md-header-btn viewer-chrome-persisted"
                  id="btn-edit"
                  type="button"
                  onClick={() => {
                    getHomeEntryShell()?.forceRecoverA?.('leave-host');
                    enterEditMode();
                  }}
                >
                  ✏️ Edit
                </button>
                <button className="md-header-btn viewer-chrome-persisted" id="btn-add-comment">
                  💬 Comment
                </button>
                <button
                  className="md-header-btn primary viewer-chrome-persisted"
                  id="btn-save"
                  type="button"
                  style={{ display: 'none' }}
                  onClick={() => void saveDoc()}
                >
                  💾 Save
                </button>
                <button
                  className="md-header-btn viewer-chrome-persisted"
                  id="btn-cancel-edit"
                  type="button"
                  style={{ display: 'none' }}
                  onClick={() => exitEditMode(false)}
                >
                  Cancel
                </button>
                <button
                  className="md-header-btn viewer-chrome-persisted"
                  id="btn-panel-commit"
                  type="button"
                  style={{ display: 'none' }}
                  onClick={() => void openCommitDialog()}
                >
                  ● Pending commit
                </button>
                <button id="md-close" type="button" className="md-header-btn viewer-close" onClick={() => void closeModal()}>
                  ✕ Close
                </button>
              </div>
              <div id="md-commit-bar" className="viewer-commit-bar viewer-chrome-persisted"></div>
              <div
                id="md-links-bar"
                className="viewer-chrome-persisted"
                style={{
                  display: 'none',
                  padding: '8px 20px',
                  borderBottom: '1px solid #d0d7de',
                  alignItems: 'center',
                  flexWrap: 'wrap',
                  gap: '4px 0',
                }}
              ></div>
              <div
                id="md-tags-bar"
                className="viewer-chrome-persisted"
                style={{
                  display: 'none',
                  padding: '8px 20px',
                  borderBottom: '1px solid #d0d7de',
                  flexWrap: 'wrap',
                  alignItems: 'center',
                  gap: '4px 0',
                }}
              ></div>
              <div id="md-content-row" className="viewer-content-row">
                <div id="md-body" className="viewer-body"></div>
                <textarea id="md-edit-area" className="viewer-edit-area" style={{ display: 'none' }} spellCheck={false}></textarea>
                <div id="knowledge-panel" className="ks-collapsed viewer-chrome-persisted"></div>
                <div id="comment-float-nav" className="viewer-chrome-persisted"></div>
              </div>
            </div>
          </div>
          <div
            id="feed-view"
            style={{ display: 'none', padding: '20px', overflowY: 'auto', height: '100%', boxSizing: 'border-box' }}
          ></div>
        </main>
      </div>
    </>
  );
}
