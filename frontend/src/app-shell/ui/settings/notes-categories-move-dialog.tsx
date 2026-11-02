import { useEffect, useSyncExternalStore } from 'react';
import { OverlayDismissButton } from '../../../shared/overlay-dismiss-button.tsx';
import {
  backMoveTarget,
  cancelMoveProject,
  chooseMoveTarget,
  confirmMoveProject,
  selectMoveWorkspace,
} from '../../commands/settings/notes-categories-move.ts';
import { notesCategoriesStore } from '../../state/settings/notes-categories.ts';

/** What the confirm step tells the user. The target project already exists. */
export function describeProjectMove(
  title: string,
  count: number,
  targetTitle: string,
  targetWorkspaceName: string,
): string {
  if (count <= 0) return `“${title}” has no documents. It will be deleted. This cannot be undone.`;
  const docs = count === 1 ? '1 document' : `${count} documents`;
  return `${docs} in “${title}” will be moved to “${targetTitle}” in “${targetWorkspaceName}”, then “${title}” is deleted. This cannot be undone.`;
}

/** Pick a workspace and an existing project, then confirm before the source project is deleted. */
export function NotesCategoriesMoveDialog() {
  const snap = useSyncExternalStore(notesCategoriesStore.subscribe, notesCategoriesStore.getSnapshot);
  const source = snap.rows.find((row) => row.id === snap.moveId);
  const target = snap.moveProjects.find((row) => row.id === snap.moveTargetId);
  const targetWorkspace = snap.moveWorkspaces.find((row) => row.id === snap.moveWorkspaceId);
  const open = Boolean(snap.moveId);
  const confirming = Boolean(target && targetWorkspace);
  const sourceTitle = source?.title || source?.id || 'this project';

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      e.preventDefault();
      void cancelMoveProject();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open]);

  return (
    <div
      id="notes-cat-move-dialog"
      className={open ? 'open' : undefined}
      role="presentation"
    >
      {open ? (
        <div id="notes-cat-move-dialog-box" role="dialog" aria-modal="true" aria-labelledby="notes-cat-move-dialog-title">
          <div id="notes-cat-move-dialog-header">
            <h3 id="notes-cat-move-dialog-title">Move project</h3>
            <OverlayDismissButton
              id="btn-notes-cat-move-close"
              disabled={snap.busy}
              label="Close"
              onClick={() => cancelMoveProject()}
            />
          </div>
          <div id="notes-cat-move-dialog-body">
            {confirming && target && targetWorkspace ? (
              <p id="notes-cat-move-copy" className="notes-cat-confirm-copy">
                {describeProjectMove(sourceTitle, source?.doc_count ?? 0, target.title || target.id, targetWorkspace.name)}
              </p>
            ) : (
              <>
                <div id="notes-cat-move-workspaces">
                  {snap.moveWorkspaces.map((workspace) => (
                    <button
                      key={workspace.id}
                      type="button"
                      className={workspace.id === snap.moveWorkspaceId ? 'selected' : undefined}
                      disabled={snap.busy || snap.moveLoading}
                      onClick={() => void selectMoveWorkspace(workspace.id)}
                    >
                      {workspace.name}
                    </button>
                  ))}
                </div>
                <div id="notes-cat-move-list">
                  {snap.moveProjects.map((project) => (
                    <button
                      key={project.id}
                      type="button"
                      disabled={snap.busy || snap.moveLoading}
                      onClick={() => chooseMoveTarget(project.id)}
                    >
                      <span className="notes-cat-move-name">{project.title || project.id}</span>
                      {project.description.trim() ? (
                        <span className="notes-cat-move-desc">{project.description}</span>
                      ) : null}
                    </button>
                  ))}
                </div>
                {!snap.moveLoading && snap.moveProjects.length === 0 && !snap.moveError ? (
                  <p className="notes-cat-confirm-copy">No other project in this workspace.</p>
                ) : null}
              </>
            )}
            <div id="notes-cat-move-error" className="settings-result notes-cat-error">
              {snap.moveError}
            </div>
            {snap.busy ? <p className="notes-cat-confirm-copy">Moving…</p> : null}
            <div className="notes-cat-edit-actions">
              {snap.busy ? (
                <button type="button" className="btn-settings-ghost" disabled={snap.cancelling} onClick={() => void cancelMoveProject()}>
                  {snap.cancelling ? 'Stopping…' : 'Cancel'}
                </button>
              ) : confirming ? (
                <button type="button" className="btn-settings-ghost" onClick={() => backMoveTarget()}>
                  Back
                </button>
              ) : (
                <button type="button" className="btn-settings-ghost" onClick={() => void cancelMoveProject()}>
                  Cancel
                </button>
              )}
              {confirming ? (
                <button
                  id="btn-notes-cat-move-confirm"
                  type="button"
                  className="sediment-kb-delete-btn notes-cat-delete-confirm"
                  disabled={snap.busy}
                  onClick={() => void confirmMoveProject(snap.workspaceId)}
                >
                  {snap.busy ? 'Moving…' : 'Move'}
                </button>
              ) : null}
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}
