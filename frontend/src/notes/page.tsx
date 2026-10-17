import { useEffect, useLayoutEffect, useRef, useSyncExternalStore } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { getEntryId } from '../host/state.ts';
import { useHostState } from './state/host.ts';
import { getHomeEntryShell } from '../home-entry-shell/access.ts';
import { OverlayDismissButton } from '../shared/overlay-dismiss-button.tsx';
import { ViewerHeaderIcon } from '../shared/viewer-header-icons.tsx';
import { NotesCommentFloatNav, NotesCommentsBar, NotesDeleteZone, openCommentDialog } from './ui/comments.tsx';
import { NotesDocList } from './ui/cards.tsx';
import { categoryLabel, docCategoryCatalogStore } from '../host/doc-categories-catalog.ts';
import { openSourceTypePicker } from './commands/cards.ts';
import { NotesLinksBar } from './ui/links-bar.tsx';
import { NotesSidebar } from './ui/sidebar.tsx';
import { NotesTagsBar } from './ui/tags-bar.tsx';
import { copyNoteReference, openNoteInChat } from './commands/open-in-chat.ts';
import type { HostNoteEntry } from '../host/state.ts';
import { closeModal, enterEditMode, exitEditMode, saveDoc, switchLang } from './viewer.ts';
import {
  findNotesDateGroup,
  notesDateHeadingText,
  resolveNotesListDate,
} from './commands/viewer/outlet.ts';
import { renderDocBody } from './ui/viewer/body.tsx';
import { SparkSearch } from './ui/search.tsx';
import { startCreateDocument } from './commands/create-document.ts';
import { CreateTargetDialog } from './ui/create-target-dialog.tsx';
import { normalizeGroupKey } from './state/dir-tree.ts';
import { entryGroupKey, isDirectoryView } from './commands/workspace.ts';

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
  return <div id="status" style={{ display: 'none' }} />;
}

/** What the toolbar shows while nothing is selected; it sits in the heading's place, beside the search. */
function emptySelectionHint(): string {
  return isDirectoryView() ? 'Select a document in the tree' : 'Select a date to view documents';
}

function NotesDateHeading({ text, hint }: { text: string; hint?: boolean }) {
  return (
    <div id="date-heading" className={hint ? 'date-heading-hint' : undefined}>
      {text}
    </div>
  );
}

function copyReference(entry: HostNoteEntry | null) {
  const btn = document.getElementById('btn-copy-path');
  if (!btn) return;
  void copyNoteReference(entry).then((copied) => {
    if (!copied) return;
    const icon = btn.innerHTML;
    btn.textContent = '✓';
    setTimeout(() => {
      btn.innerHTML = icon;
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
    const id = getEntryId(entry) || '';
    void renderDocBody(rawText, layer, id);
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
  useSyncExternalStore(
    docCategoryCatalogStore.subscribe,
    docCategoryCatalogStore.getSnapshot,
    docCategoryCatalogStore.getSnapshot,
  );
  const viewer = host.viewer;
  const mode = notesViewMode(routeParams, viewer);
  const creating = mode === 'create' || Boolean(viewer.createSession);
  const editing = viewer.editing || creating;
  const entry = viewer.entry;
  const sourceLabel = categoryLabel(entry?.source_type);
  const hasZh = Boolean(entry?.translations?.zh);
  const kbUrl = '';
  const fileSize = viewer.fileSize || (viewer.rawText ? formatBytes(viewer.rawText) : '');
  const loadFailed = Boolean(viewer.loadError);

  const dateStr = entry
    ? entryGroupKey(entry)
    : normalizeGroupKey(host.ui.activeDate) || normalizeGroupKey(routeParams.date);
  const panelTitle = notesDateHeadingText(dateStr, findNotesDateGroup(dateStr));

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
        <div id="md-header" className="viewer-header" data-tauri-drag-region="deep">
          <span id="md-panel-title" className="viewer-panel-title">
            {panelTitle}
          </span>
          <div className="viewer-header-actions">
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
          <button
            type="button"
            className="md-header-btn viewer-chrome-persisted"
            id="md-source-type"
            aria-haspopup="listbox"
            aria-expanded="false"
            aria-label="Change category"
            data-tip="Change category"
            style={{ display: entry && !creating ? undefined : 'none' }}
            onClick={(event) => {
              if (entry) openSourceTypePicker(event.currentTarget, entry);
            }}
          >
            {sourceLabel || 'Category'}
          </button>
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
            data-tip="Go to workspace"
            onClick={() => {
              if (kbUrl) window.open(kbUrl, '_blank', 'noopener,noreferrer');
            }}
          >
            📚 Workspace
          </button>
          <button
            className="md-header-btn viewer-chrome-persisted"
            id="btn-copy-path"
            type="button"
            style={{ display: entry && !creating ? undefined : 'none' }}
            data-tip="Copy reference"
            aria-label="Copy reference"
            onClick={() => copyReference(entry)}
          >
            <ViewerHeaderIcon name="copy" />
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
            <ViewerHeaderIcon name="edit" />
          </button>
          <button
            className="md-header-btn viewer-chrome-persisted"
            id="btn-add-comment"
            type="button"
            style={{ display: creating || editing || !entry ? 'none' : undefined }}
            onClick={() => {
              const layerData = (viewer.annotation.raw || {}) as { comments?: unknown[] };
              const nextIdx = (layerData.comments || []).length + 1;
              void openCommentDialog(null, null, null, nextIdx);
            }}
          >
            <ViewerHeaderIcon name="comment" />
          </button>
          <button
            className="md-header-btn viewer-chrome-persisted"
            id="btn-save"
            type="button"
            style={{ display: editing && !creating ? undefined : 'none' }}
            disabled={viewer.saving}
            onClick={() => void saveDoc()}
          >
            {viewer.saving ? 'Saving…' : 'Save'}
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
          <button
            className="md-header-btn viewer-chrome-persisted"
            id="btn-open-in-chat"
            type="button"
            data-tip="Open in chat"
            aria-label="Open in chat"
            style={{ display: entry && !creating ? undefined : 'none' }}
            onClick={(event) => {
              void openNoteInChat(entry, event.currentTarget).catch((err) => {
                alert(err instanceof Error ? err.message : String(err));
              });
            }}
          >
            <ViewerHeaderIcon name="chat" filled />
          </button>
          <OverlayDismissButton id="md-close" label="Close" onClick={() => void closeModal()} />
          </div>
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

  const { date, group } = resolveNotesListDate(host.ui.activeDate, routeParams.date);
  const directory = isDirectoryView();
  const emptyFilter = Boolean(!group && host.ui.activeTopic && !directory);
  const emptyFolder = Boolean(directory && group && group.entries.length === 0);
  const selectedHeading = emptyFilter
    ? 'No matching items'
    : notesDateHeadingText(date, group);
  const headingIsHint = !selectedHeading;
  const headingText = selectedHeading || emptySelectionHint();

  return (
    <>
      <div
        className="page-toolbar notes-page-toolbar"
        hidden={showOutlet}
        data-tauri-drag-region="deep"
      >
        <NotesDateHeading text={headingText} hint={headingIsHint} />
        <SparkSearch />
      </div>
      <button
        type="button"
        className="notes-new-fab"
        data-role="create-note"
        hidden={showOutlet}
        disabled={Object.keys(host.index.topicTitles || {}).length === 0}
        onClick={() => startCreateDocument()}
      >
        + New document
      </button>
      <NotesStatus />
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
            empty={emptyFolder}
            emptyMessage={emptyFolder ? 'No documents directly in this folder. Open it in the tree.' : undefined}
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
      <CreateTargetDialog />
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
