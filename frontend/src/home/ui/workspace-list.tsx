import { useState, type Ref } from 'react';
import type { HubWorkspaceFile } from '../state/store.ts';

export function WorkspaceList({
  items,
  canRemove,
  onOpen,
  onRemove,
  open,
  rootRef,
  onToggle,
}: {
  items: HubWorkspaceFile[];
  canRemove: boolean;
  onOpen: (item: HubWorkspaceFile) => void;
  onRemove: (path: string) => void;
  open: boolean;
  rootRef: Ref<HTMLDetailsElement>;
  onToggle: (event: { currentTarget: HTMLDetailsElement }) => void;
}) {
  const [pendingPath, setPendingPath] = useState('');
  if (!items.length) return null;
  const pending = items.find((item) => item.path === pendingPath) || null;
  return (
    <>
      <details
        ref={rootRef}
        className="home-chat-staged"
        data-role="workspace-list"
        data-open={open ? 'true' : 'false'}
        open={open}
        onToggle={onToggle}
      >
        <summary className="home-chat-staged-summary">Workspace</summary>
        {items.map((item) => (
          <div key={item.path} className="home-chat-staged-row" data-role="workspace-row">
            <button
              type="button"
              className="home-chat-staged-item"
              data-role="workspace-item"
              data-workspace-path={item.path}
              onClick={() => onOpen(item)}
            >
              <span className="home-chat-staged-title" data-role="workspace-title">
                {item.title}
              </span>
            </button>
            {canRemove ? (
              <button
                type="button"
                className="home-chat-staged-remove"
                data-role="remove-workspace"
                data-workspace-path={item.path}
                aria-label={`Delete ${item.title} from Workspace`}
                title="Delete from Workspace"
                onClick={() => setPendingPath(item.path)}
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
          data-role="remove-workspace-confirm"
          role="dialog"
          aria-modal="true"
          aria-label="Delete workspace file confirmation"
        >
          <div
            className="home-chat-delete-confirm-backdrop"
            data-role="cancel-remove-workspace"
            onClick={() => setPendingPath('')}
          />
          <div className="home-chat-delete-confirm-panel">
            <h4 className="home-chat-delete-confirm-title">Delete from Workspace</h4>
            <p className="home-chat-delete-confirm-body">
              Delete “{pending.title}” from Workspace? The file is removed.
            </p>
            <div className="home-chat-delete-confirm-actions">
              <button
                type="button"
                className="md-header-btn"
                data-role="cancel-remove-workspace"
                onClick={() => setPendingPath('')}
              >
                Cancel
              </button>
              <button
                type="button"
                className="md-header-btn home-chat-btn-danger"
                data-role="confirm-remove-workspace"
                onClick={() => {
                  const path = pending.path;
                  setPendingPath('');
                  if (path) onRemove(path);
                }}
              >
                Delete
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </>
  );
}
