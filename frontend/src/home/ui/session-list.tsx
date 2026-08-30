import { useState } from 'react';
import { sessionListLabel, type HubSession } from '../state/store.ts';

function TrashIcon() {
  return (
    <svg className="home-chat-session-delete-icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <path
        fill="currentColor"
        d="M9 3h6l1 2h4v2H4V5h4l1-2zm1 6h2v8h-2V9zm4 0h2v8h-2V9zM7 9h2v8H7V9z"
      />
    </svg>
  );
}

export function SessionList({
  sessions,
  currentSessionId,
  progressByChat,
  inFlightIds,
  onSelect,
  onDelete,
}: {
  sessions: HubSession[];
  currentSessionId: string;
  progressByChat: Record<string, string>;
  inFlightIds: string[];
  onSelect: (id: string) => void;
  onDelete: (id: string) => void;
}) {
  const [pendingId, setPendingId] = useState('');
  if (!sessions.length) {
    return <p className="home-chat-sessions-empty">No conversations yet.</p>;
  }
  const pending = sessions.find((s) => String(s.session_id || '') === pendingId);
  return (
    <>
      {sessions.map((s) => {
        const id = String(s.session_id || '');
        const title = sessionListLabel(s);
        const active = id && id === currentSessionId ? ' is-active' : '';
        const flying = Boolean(progressByChat[id]);
        const inFlight = inFlightIds.includes(id);
        return (
          <div key={id || title} className={`home-chat-session${active}`} role="listitem">
            <button
              type="button"
              className="home-chat-session-open"
              data-session-id={id}
              onClick={() => {
                if (id) onSelect(id);
              }}
            >
              {title}
              {flying ? (
                <span className="home-chat-session-progress" aria-hidden="true">
                  …
                </span>
              ) : null}
            </button>
            {inFlight ? null : (
              <button
                type="button"
                className="home-chat-session-delete"
                data-role="delete-session"
                data-session-id={id}
                aria-label="Delete conversation"
                title="Delete conversation"
                onClick={() => {
                  if (id) setPendingId(id);
                }}
              >
                <TrashIcon />
              </button>
            )}
          </div>
        );
      })}
      {pending ? (
        <div
          className="home-chat-delete-confirm"
          data-role="delete-session-confirm"
          role="dialog"
          aria-modal="true"
          aria-label="Delete conversation confirmation"
        >
          <div
            className="home-chat-delete-confirm-backdrop"
            data-role="cancel-delete-session"
            onClick={() => setPendingId('')}
          />
          <div className="home-chat-delete-confirm-panel">
            <h4 className="home-chat-delete-confirm-title">Delete conversation</h4>
            <p className="home-chat-delete-confirm-body">
              Delete “{sessionListLabel(pending)}”? This cannot be undone.
            </p>
            <div className="home-chat-delete-confirm-actions">
              <button
                type="button"
                className="md-header-btn"
                data-role="cancel-delete-session"
                onClick={() => setPendingId('')}
              >
                Cancel
              </button>
              <button
                type="button"
                className="md-header-btn home-chat-btn-danger"
                data-role="confirm-delete-session"
                onClick={() => {
                  const id = String(pending.session_id || '');
                  setPendingId('');
                  if (id) onDelete(id);
                }}
              >
                Confirm delete
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </>
  );
}
