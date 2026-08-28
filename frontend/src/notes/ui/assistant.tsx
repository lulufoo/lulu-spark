import { useCallback, useEffect, useRef, useState } from 'react';
import { flushSync } from 'react-dom';
import { createRoot, type Root } from 'react-dom/client';
import { filenameFromPath, slugToTitle } from '../../shared/utils.ts';
import {
  loadAssistantNotes,
  selectTopNotesByCreatedAt,
  type AssistantNote,
} from '../commands/assistant.ts';

export { loadAssistantNotes, selectTopNotesByCreatedAt };

const UNAVAILABLE_MSG = 'List temporarily unavailable — try again later';

function noteLabel(entry: AssistantNote) {
  if (entry.title) return String(entry.title);
  if (entry.common_path) {
    return slugToTitle(filenameFromPath(entry.common_path));
  }
  return 'Untitled note';
}

function EmptyState() {
  return (
    <div className="note-assistant-empty note-assistant-state">
      <p className="note-assistant-state-title">No notes yet</p>
      <p className="note-assistant-state-detail">Create below; latest notes appear here</p>
    </div>
  );
}

function ErrorEmpty({ message = UNAVAILABLE_MSG }: { message?: string }) {
  return (
    <div className="note-assistant-empty note-assistant-state note-assistant-state--error">
      <p className="note-assistant-state-title">Unable to load</p>
      <p className="note-assistant-state-detail">{message}</p>
    </div>
  );
}

function NoteList({
  notes,
  onOpen,
}: {
  notes: AssistantNote[];
  onOpen: (commonPath: string) => void;
}) {
  return (
    <ul className="note-assistant-list">
      {notes.map((entry) => {
        const path = entry.common_path ?? '';
        const label = noteLabel(entry);
        return (
          <li key={path || label} className="note-assistant-item" data-common-path={path}>
            <button
              type="button"
              className="note-assistant-item-link"
              data-common-path={path}
              onClick={() => {
                if (path) onOpen(path);
              }}
            >
              <span className="note-assistant-item-summary">{label}</span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}

function CreateButton({ onCreate }: { onCreate: () => void }) {
  return (
    <button type="button" className="note-assistant-create" onClick={onCreate}>
      New note
    </button>
  );
}

function bindFocusRefresh(refresh: () => unknown) {
  const onFocus = () => {
    void refresh();
  };
  const onVisibility = () => {
    if (document.visibilityState === 'visible') {
      void refresh();
    }
  };
  window.addEventListener('focus', onFocus);
  document.addEventListener('visibilitychange', onVisibility);
  return () => {
    window.removeEventListener('focus', onFocus);
    document.removeEventListener('visibilitychange', onVisibility);
  };
}

export function NoteAssistant({
  autoLoad = true,
  openCreateNote,
  handleRef,
}: {
  autoLoad?: boolean;
  openCreateNote?: (opts?: object) => void | Promise<void>;
  handleRef?: { current: { refresh: () => Promise<void> } };
}) {
  const [loading, setLoading] = useState(false);
  const [notes, setNotes] = useState<AssistantNote[]>([]);
  const [error, setError] = useState<string | null>(null);
  const refreshPromise = useRef<Promise<void> | null>(null);
  const notesRef = useRef(notes);
  notesRef.current = notes;
  const showCreate = typeof openCreateNote === 'function';

  const refresh = useCallback(async () => {
    if (refreshPromise.current) return refreshPromise.current;
    refreshPromise.current = (async () => {
      if (!notesRef.current.length) setLoading(true);
      try {
        const entries = await loadAssistantNotes();
        setNotes(selectTopNotesByCreatedAt(entries));
        setError(null);
      } catch {
        setNotes([]);
        setError(UNAVAILABLE_MSG);
      } finally {
        setLoading(false);
      }
    })().finally(() => {
      refreshPromise.current = null;
    });
    return refreshPromise.current;
  }, []);

  useEffect(() => {
    if (handleRef) handleRef.current = { refresh };
  }, [handleRef, refresh]);

  useEffect(() => {
    const dispose = bindFocusRefresh(refresh);
    if (autoLoad) void refresh();
    return dispose;
  }, [autoLoad, refresh]);

  if (loading) {
    return <div className="note-assistant-loading">Loading…</div>;
  }
  if (error) {
    return (
      <>
        <ErrorEmpty message={error} />
        {showCreate ? (
          <CreateButton
            onCreate={() => {
              void openCreateNote?.();
            }}
          />
        ) : null}
      </>
    );
  }
  if (!notes.length) {
    return (
      <>
        <EmptyState />
        {showCreate ? (
          <CreateButton
            onCreate={() => {
              void openCreateNote?.();
            }}
          />
        ) : null}
      </>
    );
  }
  return (
    <>
      <NoteList
        notes={notes}
        onOpen={(commonPath) => {
          document.dispatchEvent(new CustomEvent('cta:open-entry', { detail: { common_path: commonPath } }));
        }}
      />
      {showCreate ? (
        <CreateButton
          onCreate={() => {
            void openCreateNote?.();
          }}
        />
      ) : null}
    </>
  );
}

export function mountNoteAssistant(
  root: HTMLElement,
  opts: { autoLoad?: boolean; openCreateNote?: (opts?: object) => void | Promise<void> } = {},
) {
  const handleRef = { current: { refresh: () => Promise.resolve() } };
  const reactRoot: Root = createRoot(root);
  flushSync(() => {
    reactRoot.render(<NoteAssistant {...opts} handleRef={handleRef} />);
  });
  return {
    dispose() {
      flushSync(() => {
        reactRoot.unmount();
      });
      root.innerHTML = '';
    },
    refresh() {
      return handleRef.current.refresh();
    },
  };
}

/** Brand label retained for copy-lock / a11y (overlay title comes from EntryConfig). */
export const NOTES_CONTENT_LABEL = 'Notes Assistant';

export function createNotesContentAdapter() {
  return {
    mount(
      slotEl: HTMLElement,
      ctx: { host?: { openCreateNote?: (opts?: object) => void | Promise<void> } } = {},
    ) {
      const host = ctx.host ?? {};
      slotEl.setAttribute('aria-label', NOTES_CONTENT_LABEL);
      const panel = mountNoteAssistant(slotEl, {
        autoLoad: true,
        openCreateNote: typeof host.openCreateNote === 'function' ? host.openCreateNote : undefined,
      });
      return {
        unmount() {
          panel.dispose();
          slotEl.removeAttribute('aria-label');
        },
      };
    },
  };
}
