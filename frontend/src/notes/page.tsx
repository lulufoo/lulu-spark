import { useEffect, useLayoutEffect, useRef } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { getActivePath } from '../knowledge/state/path.ts';
import { KnowledgeSearchHost } from '../knowledge/ui/knowledge-search.tsx';
import { getGithubUserUrl, notesFileRelPath, sparkGithubBlobBase } from '../host/constants.ts';
import { useHostState } from './state/host.ts';
import { getHomeEntryShell } from '../home-entry-shell/access.ts';
import { OverlayDismissButton } from '../shared/overlay-dismiss-button.tsx';
import { formatDate } from '../shared/utils.ts';
import { NotesCommentFloatNav, NotesCommentsBar, NotesDeleteZone, openCommentDialog } from './ui/comments.tsx';
import { NotesDocList } from './ui/cards.tsx';
import { NotesLinksBar } from './ui/links-bar.tsx';
import { NotesSidebar } from './ui/sidebar.tsx';
import { NotesTagsBar } from './ui/tags-bar.tsx';
import { clearTagFilter } from './commands/sidebar.ts';
import { initSidebarResize } from './ui/sidebar-resize.ts';
import { openNoteInChat } from './commands/open-in-chat.ts';
import { closeModal, enterEditMode, exitEditMode, saveDoc, switchLang } from './viewer.ts';
import { renderDocBody } from './ui/viewer/body.tsx';

export { NotesSidebar };

function formatBytes(text: string) {
  const bytes = new Blob([text]).size;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function notesViewMode(
  routeParams: Record<string, string>,
  viewer: { createSession?: unknown; outletMode?: string },
) {
  if (routeParams.note) {
    return viewer.outletMode === 'safe-empty' ? 'safe-empty' : 'open';
  }
  if (viewer.createSession) return 'create';
  if (viewer.outletMode === 'create' || viewer.outletMode === 'safe-empty' || viewer.outletMode === 'open') {
    return viewer.outletMode;
  }
  return 'list';
}

function NotesStatus() {
  const host = useHostState();
  const err = host.ui.loadError;
  if (err) {
    return (
      <div id="status">
        <div className="error-msg">
          {err}
          <br />
          <button type="button" onClick={() => document.dispatchEvent(new CustomEvent('cta:reload'))}>
            Retry
          </button>
        </div>
      </div>
    );
  }
  if (host.ui.activeDate) return <div id="status" style={{ display: 'none' }} />;
  return (
    <div id="status">
      <div style={{ color: '#8c959f', fontSize: '13px' }}>← Select a date to view documents</div>
    </div>
  );
}

function NotesDateHeading() {
  const host = useHostState();
  const date = host.ui.activeDate;
  const groups = host.index.filteredGroups || [];
  const group = date ? groups.find((g) => g.date === date) : null;
  const empty = groups.length === 0 && (host.ui.activeTopic || host.ui.activeTagKey);
  if (empty) {
    return (
      <div id="date-heading" style={{ display: '' }}>
        No matching items
      </div>
    );
  }
  if (!date || !group) return <div id="date-heading" style={{ display: 'none' }} />;
  const d = formatDate(date);
  return (
    <div id="date-heading" style={{ display: '' }}>
      {d.full}  ·  {group.entries.length} items
    </div>
  );
}

function NotesTagChip() {
  const host = useHostState();
  const key = host.ui.activeTagKey;
  if (!key) return null;
  const registry = host.index.tagsRegistry;
  const label = registry.keys?.[key]?.value || key;
  return (
    <span id="tag-filter-chip" className="tag-filter-chip">
      <span>{`Tag: ${label} `}</span>
      <button type="button" className="tag-filter-chip-clear" title="Clear tag filter" onClick={() => clearTagFilter()}>
        ×
      </button>
    </span>
  );
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
  sparkRoot: string,
) {
  if (!entry) return;
  const activePath = getActivePath(entry, lang || '', layer) || '';
  const relPath = notesFileRelPath(layer, activePath);
  const fullPath = sparkRoot ? `${sparkRoot}/${relPath}` : relPath;
  const btn = document.getElementById('btn-copy-path');
  if (!btn) return;
  void navigator.clipboard.writeText(fullPath).then(() => {
    btn.textContent = '✓';
    setTimeout(() => {
      btn.textContent = '📂';
    }, 1200);
  });
}

function NotesReaderBody() {
  const host = useHostState();
  const viewer = host.viewer;
  const bodyRef = useRef<HTMLDivElement | null>(null);
  const paintKey = viewer.bodyPaintKey;
  const rawText = viewer.rawText;
  const layer = viewer.layer || 'raw';
  const entry = viewer.entry;
  const loading = viewer.loading;
  const loadError = viewer.loadError;
  const creating = Boolean(viewer.createSession);
  const editing = viewer.editing || creating;

  useLayoutEffect(() => {
    const body = bodyRef.current;
    if (!body || creating || editing) return;
    if (loading) {
      body.innerHTML = '<div style="color:#8c959f;padding:20px;font-size:13px">Loading…</div>';
      return;
    }
    if (loadError) {
      body.textContent = '';
      const wrap = document.createElement('div');
      wrap.style.color = '#7d4e00';
      wrap.style.padding = '20px';
      wrap.textContent = `Could not load file: ${loadError}`;
      body.appendChild(wrap);
      return;
    }
    if (!entry || !rawText) {
      body.innerHTML = '';
      return;
    }
    const activePath = getActivePath(entry, viewer.lang || '', layer) || entry.common_path;
    void renderDocBody(rawText, layer, activePath);
  }, [paintKey, rawText, layer, entry, loading, loadError, creating, editing, viewer.lang]);

  return (
    <div className="viewer-body" style={creating || editing ? { display: 'none' } : undefined}>
      <NotesCommentsBar key="comments" />
      <div id="md-body" key="md-body" ref={bodyRef} />
      <NotesDeleteZone key="delete" />
    </div>
  );
}

function NotesOutletChrome({ routeParams }: { routeParams: Record<string, string> }) {
  const host = useHostState();
  const viewer = host.viewer;
  const mode = notesViewMode(routeParams, viewer);
  const creating = mode === 'create' || Boolean(viewer.createSession);
  const editing = viewer.editing || creating;
  const entry = viewer.entry;
  const layer = viewer.layer || 'raw';
  const hasZh = Boolean(entry?.translations?.zh);
  const ghBase = sparkGithubBlobBase(getGithubUserUrl(), host.ui.sparkRoot);
  const activePath = entry ? getActivePath(entry, viewer.lang || '', layer) || '' : '';
  const githubUrl = ghBase && entry ? `${ghBase}/${notesFileRelPath(layer, activePath)}` : '';
  const kbUrl = '';
  const relPath = entry ? notesFileRelPath(layer, activePath) : '';
  const fullPath = host.ui.sparkRoot ? `${host.ui.sparkRoot}/${relPath}` : relPath;
  const fileSize = viewer.fileSize || (viewer.rawText ? formatBytes(viewer.rawText) : '');
  const loadFailed = Boolean(viewer.loadError);

  const dateStr = String(entry?.created_at || host.ui.activeDate || routeParams.date || '').slice(0, 8);
  const groups = host.index.filteredGroups || host.index.groupedByDate || [];
  const group = /^\d{8}$/.test(dateStr) ? groups.find((g) => g.date === dateStr) : null;
  const count = group?.entries?.length;
  const panelTitle =
    /^\d{8}$/.test(dateStr)
      ? count != null
        ? `${formatDate(dateStr).full}  ·  ${count} items`
        : formatDate(dateStr).full
      : '';

  return (
    <>
      <div
        id="note-outlet-message"
        className="note-outlet-message"
        hidden={mode !== 'safe-empty'}
      >
        {mode === 'safe-empty' ? viewer.outletMessage : ''}
      </div>
      <div id="md-panel" className="viewer-panel" hidden={mode === 'safe-empty'}>
        <div id="md-header" className="viewer-header">
          <span id="md-panel-title" className="viewer-panel-title">
            {panelTitle}
          </span>
          <div
            id="md-lang-bar"
            className="viewer-chrome-persisted"
            style={{ display: hasZh && !creating ? 'flex' : 'none', flexShrink: 0, alignItems: 'center', gap: '2px' }}
          >
            <button
              className={`md-header-btn${viewer.lang !== 'zh' ? ' active' : ''}`}
              id="btn-lang-en"
              type="button"
              onClick={() => void switchLang('en')}
            >
              EN
            </button>
            <button
              className={`md-header-btn${viewer.lang === 'zh' ? ' active' : ''}`}
              id="btn-lang-zh"
              type="button"
              onClick={() => void switchLang('zh')}
            >
              ZH
            </button>
          </div>
          <span
            id="md-file-size"
            className="viewer-chrome-persisted"
            style={{ fontSize: '10px', color: '#8c959f', flexShrink: 0, display: creating ? 'none' : undefined }}
          >
            {fileSize}
          </span>
          <button
            className="md-header-btn viewer-chrome-persisted"
            id="btn-goto-kb"
            type="button"
            style={{ display: kbUrl && !creating ? undefined : 'none' }}
            title="Go to Knowledge"
            onClick={() => {
              if (kbUrl) window.open(kbUrl, '_blank', 'noopener,noreferrer');
            }}
          >
            📚 Knowledge
          </button>
          <button
            className="md-header-btn viewer-chrome-persisted"
            id="btn-copy-http"
            type="button"
            data-tip={githubUrl}
            data-url={githubUrl}
            style={{ display: githubUrl && !creating ? undefined : 'none' }}
            onClick={copyHttpLink}
          >
            🌐
          </button>
          <button
            className="md-header-btn viewer-chrome-persisted"
            id="btn-copy-path"
            type="button"
            data-tip={fullPath}
            style={{ display: entry && !creating ? undefined : 'none' }}
            onClick={() =>
              copyLocalPath(entry, viewer.lang, viewer.layer, host.ui.sparkRoot)
            }
          >
            📁
          </button>
          <button
            className="md-header-btn viewer-chrome-persisted"
            id="btn-open-in-chat"
            type="button"
            title="Open in chat"
            aria-label="Open in chat"
            style={{ display: entry && !creating ? undefined : 'none' }}
            onClick={(event) => {
              void openNoteInChat(
                entry,
                viewer.lang,
                viewer.layer,
                host.ui.sparkRoot,
                event.currentTarget,
              ).catch((err) => {
                alert(err instanceof Error ? err.message : String(err));
              });
            }}
          >
            🗨️
          </button>
          <button
            className="md-header-btn viewer-chrome-persisted"
            id="btn-edit"
            type="button"
            style={{ display: creating || editing || loadFailed || !entry ? 'none' : undefined }}
            onClick={() => {
              getHomeEntryShell()?.forceRecoverA?.('leave-host');
              enterEditMode();
            }}
          >
            ✏️ Edit
          </button>
          <button
            className="md-header-btn viewer-chrome-persisted"
            id="btn-add-comment"
            type="button"
            style={{ display: creating || editing || !entry ? 'none' : undefined }}
            onClick={() => {
              const layerData = (viewer.annotation[viewer.layer] || {}) as { comments?: unknown[] };
              const nextIdx = (layerData.comments || []).length + 1;
              void openCommentDialog(null, null, null, nextIdx);
            }}
          >
            💬 Comment
          </button>
          <button
            className="md-header-btn primary viewer-chrome-persisted"
            id="btn-save"
            type="button"
            style={{ display: editing && !creating ? undefined : 'none' }}
            disabled={viewer.saving}
            onClick={() => void saveDoc()}
          >
            {viewer.saving ? 'Saving…' : '💾 Save'}
          </button>
          <button
            className="md-header-btn viewer-chrome-persisted"
            id="btn-cancel-edit"
            type="button"
            style={{ display: editing && !creating ? undefined : 'none' }}
            onClick={() => exitEditMode(false)}
          >
            Cancel
          </button>
          <OverlayDismissButton id="md-close" title="Close" onClick={() => void closeModal()} />
        </div>
        <NotesLinksBar />
        <NotesTagsBar />
        <div id="md-content-row" className="viewer-content-row">
          <NotesReaderBody />
          <textarea
            id="md-edit-area"
            className="viewer-edit-area"
            style={{ display: creating || editing ? undefined : 'none' }}
            spellCheck={false}
            defaultValue={viewer.rawText}
            key={`${creating ? 'create' : 'edit'}-${viewer.bodyPaintKey}`}
          />
          <KnowledgeSearchHost entry={entry} creating={creating} />
          <NotesCommentFloatNav />
        </div>
      </div>
    </>
  );
}

export function NotesMain({ routeParams }: { routeParams: Record<string, string> }) {
  const host = useHostState();
  const mode = notesViewMode(routeParams, host.viewer);
  const showOutlet = mode !== 'list';
  const listRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    initSidebarResize();
  }, []);

  useEffect(() => {
    const list = listRef.current || document.getElementById('doc-list');
    if (!list) return undefined;
    const onScroll = () => {
      if (host.ui.activeDate) {
        sessionStorage.setItem('cta_scroll_' + host.ui.activeDate, String(list.scrollTop));
      }
    };
    list.addEventListener('scroll', onScroll, { passive: true });
    return () => list.removeEventListener('scroll', onScroll);
  }, [host.ui.activeDate]);

  useLayoutEffect(() => {
    const list = listRef.current || document.getElementById('doc-list');
    if (!list || !host.ui.activeDate || showOutlet) return;
    const saved = sessionStorage.getItem('cta_scroll_' + host.ui.activeDate);
    if (saved) list.scrollTop = parseInt(saved, 10);
  }, [host.ui.activeDate, showOutlet]);

  const date = host.ui.activeDate;
  const group = date
    ? host.index.filteredGroups.find(
        (g) => g.date === date,
      )
    : null;

  return (
    <>
      <NotesStatus />
      <NotesDateHeading />
      <NotesTagChip />
      <div
        id="doc-list"
        ref={listRef}
        className="doc-list"
        style={{ display: showOutlet ? 'none' : undefined }}
      >
        {!showOutlet ? (
          <NotesDocList
            entries={group?.entries || []}
            date={date || ''}
            empty={Boolean(!group && (host.ui.activeTopic || host.ui.activeTagKey))}
          />
        ) : null}
      </div>
      <div
        id="note-outlet"
        hidden={!showOutlet}
        className={mode === 'create' ? 'is-create' : undefined}
        data-wb-mode={showOutlet ? mode : ''}
        data-note={routeParams.note || undefined}
        data-layer={routeParams.layer || undefined}
      >
        <NotesOutletChrome routeParams={routeParams} />
      </div>
    </>
  );
}

export function NotesPage({ routeParams }: { routeParams: Record<string, string> }) {
  return (
    <>
      <NotesSidebar />
      <main id="main">
        <NotesMain routeParams={routeParams} />
      </main>
    </>
  );
}

let sidebarTestRoot: Root | null = null;

/** Tests only: paint NotesSidebar into `#sidebar`. Production uses ShellPages. */
export function mountNotesSidebar(container: HTMLElement) {
  sidebarTestRoot?.unmount();
  sidebarTestRoot = createRoot(container);
  flushSync(() => {
    sidebarTestRoot!.render(<NotesSidebar />);
  });
  return {
    unmount() {
      sidebarTestRoot?.unmount();
      sidebarTestRoot = null;
    },
  };
}
