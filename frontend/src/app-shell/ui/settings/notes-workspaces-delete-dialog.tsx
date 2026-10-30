import { useEffect, useSyncExternalStore } from 'react';
import { OverlayDismissButton } from '../../../shared/overlay-dismiss-button.tsx';
import { getStateVersion, state, subscribeState } from '../../../host/state.ts';
import {
  cancelDeleteWorkspace,
  confirmDeleteWorkspace,
  describeWorkspaceDelete,
} from '../../commands/settings/notes-workspaces-delete.ts';
import { workspacesPanelStore } from '../../state/settings/workspaces-panel.ts';

/** Confirm step for deleting a workspace: states the document count, project count and destination. */
export function NotesWorkspacesDeleteDialog() {
  useSyncExternalStore(subscribeState, getStateVersion, getStateVersion);
  const snap = useSyncExternalStore(workspacesPanelStore.subscribe, workspacesPanelStore.getSnapshot);
  const row = state.workspaces.list.find((w) => w.workspace_id === snap.deleteId);
  const dest = state.workspaces.list.find((w) => w.is_default);
  const open = Boolean(row && dest);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape' || snap.busy) return;
      e.preventDefault();
      cancelDeleteWorkspace();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, snap.busy]);

  return (
    <div
      id="notes-ws-delete-dialog"
      className={open ? 'open' : undefined}
      role="presentation"
      onClick={(e) => {
        if (e.target === e.currentTarget) cancelDeleteWorkspace();
      }}
    >
      {row && dest ? (
        <div
          id="notes-ws-delete-dialog-box"
          role="dialog"
          aria-modal="true"
          aria-labelledby="notes-ws-delete-dialog-title"
        >
          <div id="notes-ws-delete-dialog-header">
            <h3 id="notes-ws-delete-dialog-title">Delete workspace</h3>
            <OverlayDismissButton
              id="btn-notes-ws-delete-close"
              disabled={snap.busy}
              label="Close"
              onClick={() => cancelDeleteWorkspace()}
            />
          </div>
          <div id="notes-ws-delete-dialog-body">
            <p id="notes-ws-delete-copy" className="notes-cat-confirm-copy">
              {describeWorkspaceDelete(
                row.workspace_name || row.workspace_id,
                snap.deleteDocCount,
                snap.deleteProjectCount,
                dest.workspace_name || dest.workspace_id,
                snap.deleteInboxName,
              )}
            </p>
            <div id="notes-ws-delete-error" className="settings-result notes-cat-error">
              {snap.deleteError}
            </div>
            <div className="notes-cat-edit-actions">
              <button type="button" className="btn-settings-ghost" disabled={snap.busy} onClick={() => cancelDeleteWorkspace()}>
                Cancel
              </button>
              <button
                id="btn-notes-ws-delete-confirm"
                type="button"
                className="sediment-kb-delete-btn notes-cat-delete-confirm"
                disabled={snap.busy}
                onClick={() => void confirmDeleteWorkspace()}
              >
                {snap.busy ? 'Deleting…' : 'Delete'}
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}
