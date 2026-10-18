import { useCallback, useEffect, useMemo, useState } from 'react';
import { state } from '../../host/state.ts';
import { cmdShortcutKey } from '../../shared/search-modal/cmd-key.ts';
import type { SearchNames, SearchScope } from '../../shared/search-modal/hit-location.ts';
import { SearchGlyph, SearchModal } from '../../shared/search-modal/search-modal.tsx';

/**
 * Notes-page search: the shared modal, scoped to the current workspace (the same value the old
 * inline search passed to `search_spark`). Only the interaction is shared with Home; the notes
 * page does not search other workspaces.
 */

/**
 * Names for the location line under a result. The notes index has the current workspace's project
 * titles; a project with no title (it falls back to its id) gives '' so no raw id is shown.
 */
export function notesSearchNames(topicTitles: Record<string, string>): SearchNames {
  return {
    workspace: () => '',
    project: (_workspaceId, projectId) => {
      const title = topicTitles[projectId];
      return title && title !== projectId ? title : '';
    },
  };
}

/**
 * Open state + ⌘K for the notes page. `active` is whether the notes route is showing: the notes
 * page stays mounted while other routes are up, so the shortcut is bound only while active, and
 * leaving the route closes the modal.
 */
export function useNotesSearch(active: boolean) {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    if (!active) {
      setOpen(false);
      return undefined;
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (cmdShortcutKey(event) !== 'k') return;
      event.preventDefault();
      setOpen((current) => !current);
    };
    document.addEventListener('keydown', onKeyDown);
    return () => document.removeEventListener('keydown', onKeyDown);
  }, [active]);

  return {
    open,
    openSearch: useCallback(() => setOpen(true), []),
    closeSearch: useCallback(() => setOpen(false), []),
  };
}

/** The toolbar's search entry: looks like the old input, opens the modal. */
export function NotesSearchButton({ onOpen }: { onOpen: () => void }) {
  return (
    <button
      type="button"
      className="notes-search-btn"
      data-role="notes-search"
      aria-label="Search this workspace"
      aria-keyshortcuts="Meta+K"
      onClick={onOpen}
    >
      <SearchGlyph className="notes-search-btn-glyph" />
      <span className="notes-search-btn-label">Search this workspace…</span>
      <kbd className="notes-search-btn-key">⌘K</kbd>
    </button>
  );
}

export function NotesSearchModal({
  open,
  topicTitles,
  onClose,
}: {
  open: boolean;
  topicTitles: Record<string, string>;
  onClose: () => void;
}) {
  const workspaceId = state.workspaces.current;
  const scope = useMemo<SearchScope>(() => ({ kind: 'workspace', workspaceId }), [workspaceId]);
  const names = useMemo(() => notesSearchNames(topicTitles || {}), [topicTitles]);
  return <SearchModal open={open} scope={scope} names={names} onClose={onClose} />;
}
