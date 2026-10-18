import type { RefObject } from 'react';
import type { ResultRow, SearchMode } from './modal-mode.ts';

export type View =
  | { kind: 'history' }
  | { kind: 'status'; text: string }
  | { kind: 'hits'; rows: ResultRow[] };

export function getRowId(i: number): string {
  return `search-modal-row-${i}`;
}

/** The list under the search box: a status line, the result rows, or the recent searches. */
export function SearchResultList({
  listId,
  listRef,
  mode,
  view,
  history,
  emptyHistoryText,
  active,
  onActive,
  onOpenRow,
  onPickHistory,
  onRemoveHistory,
}: {
  listId: string;
  listRef: RefObject<HTMLDivElement | null>;
  mode: SearchMode;
  view: View;
  history: string[];
  emptyHistoryText: string;
  active: number;
  onActive: (i: number) => void;
  onOpenRow: (row: ResultRow) => void;
  onPickHistory: (q: string) => void;
  onRemoveHistory: (q: string) => void;
}) {
  return (
    <div id={listId} className="search-modal-list" role="listbox" ref={listRef}>
      {view.kind === 'status' ? <div className="search-modal-status">{view.text}</div> : null}
      {view.kind === 'hits'
        ? view.rows.map((row, i) => (
            <div
              key={`${row.id}-${i}`}
              id={getRowId(i)}
              className="search-modal-row"
              role="option"
              aria-selected={i === active}
              data-role="search-hit"
              {...(mode === 'chats' ? { 'data-session-id': row.id } : { 'data-note-id': row.id })}
              data-active={i === active ? 'true' : undefined}
              onMouseEnter={() => onActive(i)}
              onClick={() => onOpenRow(row)}
            >
              <span className="search-modal-row-title">{row.title}</span>
              {row.sub ? <span className="search-modal-row-sub">{row.sub}</span> : null}
            </div>
          ))
        : null}
      {view.kind === 'history' && history.length === 0 ? (
        <div className="search-modal-status">{emptyHistoryText}</div>
      ) : null}
      {view.kind === 'history' && history.length > 0 ? (
        <>
          <div className="search-modal-section">Recent searches</div>
          {history.map((q, i) => (
            <div
              key={q}
              id={getRowId(i)}
              className="search-modal-row search-modal-row--history"
              role="option"
              aria-selected={i === active}
              data-role="search-history-item"
              data-q={q}
              data-active={i === active ? 'true' : undefined}
              onMouseEnter={() => onActive(i)}
              onClick={() => onPickHistory(q)}
            >
              <span className="search-modal-row-title">{q}</span>
              <button
                type="button"
                className="search-modal-history-remove"
                data-role="search-history-remove"
                data-q={q}
                data-tip="Delete"
                aria-label={`Delete “${q}” from recent searches`}
                onClick={(e) => {
                  e.stopPropagation();
                  onRemoveHistory(q);
                }}
              >
                ×
              </button>
            </div>
          ))}
        </>
      ) : null}
    </div>
  );
}
