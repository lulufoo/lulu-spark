import { useState, type Ref } from 'react';
import type { HubStagedEntry } from '../state/store.ts';

export function StagedList({
  items,
  canRemove,
  onOpen,
  onRemove,
  open,
  rootRef,
  onToggle,
}: {
  items: HubStagedEntry[];
  canRemove: boolean;
  onOpen: (item: HubStagedEntry) => void;
  onRemove: (id: string) => void;
  open: boolean;
  rootRef: Ref<HTMLDetailsElement>;
  onToggle: (event: { currentTarget: HTMLDetailsElement }) => void;
}) {
  const [pendingId, setPendingId] = useState('');
  if (!items.length) return null;
  const pending = items.find((item) => item.id === pendingId) || null;
  return (
    <>
      <details
        ref={rootRef}
        className="home-chat-staged"
        data-role="staged-list"
        data-open={open ? 'true' : 'false'}
        open={open}
        onToggle={onToggle}
      >
        <summary className="home-chat-staged-summary">Staged</summary>
        {items.map((item) => (
          <div key={item.id || item.path} className="home-chat-staged-row" data-role="staged-row">
            <button
              type="button"
              className="home-chat-staged-item"
              data-role="staged-item"
              data-staged-path={item.path}
              data-staged-id={item.id}
              onClick={() => onOpen(item)}
            >
              <span className="home-chat-staged-id" data-role="staged-id">
                {item.id}
              </span>
              <span className="home-chat-staged-title" data-role="staged-title">
                {item.title}
              </span>
            </button>
            {canRemove && item.id ? (
              <button
                type="button"
                className="home-chat-staged-remove"
                data-role="remove-staged"
                data-staged-id={item.id}
                aria-label={`Remove ${item.id} from Stage`}
                title="Remove from Stage"
                onClick={() => setPendingId(item.id)}
              >
                ×
              </button>
            ) : null}
          </div>
        ))}
      </details>
      {pending ? (
        <div
          className="home-chat-delete-confirm"
          data-role="remove-staged-confirm"
          role="dialog"
          aria-modal="true"
          aria-label="Remove staged file confirmation"
        >
          <div
            className="home-chat-delete-confirm-backdrop"
            data-role="cancel-remove-staged"
            onClick={() => setPendingId('')}
          />
          <div className="home-chat-delete-confirm-panel">
            <h4 className="home-chat-delete-confirm-title">Remove from Stage</h4>
            <p className="home-chat-delete-confirm-body">
              Remove {pending.id}
              {pending.title ? ` (“${pending.title}”)` : ''} from Stage? The note file is kept.
            </p>
            <div className="home-chat-delete-confirm-actions">
              <button
                type="button"
                className="md-header-btn"
                data-role="cancel-remove-staged"
                onClick={() => setPendingId('')}
              >
                Cancel
              </button>
              <button
                type="button"
                className="md-header-btn home-chat-btn-danger"
                data-role="confirm-remove-staged"
                onClick={() => {
                  const id = pending.id;
                  setPendingId('');
                  if (id) onRemove(id);
                }}
              >
                Remove
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </>
  );
}
