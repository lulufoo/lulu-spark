import { useSyncExternalStore } from 'react';
import {
  closeMoveProjectDialog,
  doMoveProject,
  moveProjectOpenStore,
  selectMoveWorkspace,
} from '../commands/move-project-dialog.ts';
import { moveProjectViewStore } from '../state/move-project.ts';

export { closeMoveProjectDialog, openMoveProjectDialog } from '../commands/move-project-dialog.ts';

function resultColor(kind: string) {
  if (kind === 'loading') return '#8c959f';
  if (kind === 'ok') return '#1a7f37';
  if (kind === 'err') return '#cf222e';
  return '#57606a';
}

export function MoveProjectDialog() {
  const open = useSyncExternalStore(moveProjectOpenStore.subscribe, moveProjectOpenStore.getSnapshot);
  const view = useSyncExternalStore(moveProjectViewStore.subscribe, moveProjectViewStore.getSnapshot);

  return (
    <div id="move-project-dialog" className={open ? 'open' : undefined}>
      <div id="move-project-backdrop" onClick={() => closeMoveProjectDialog()}></div>
      <div id="move-project-dialog-box">
        <h3>Move document</h3>
        <div id="move-project-result" style={{ fontSize: '12px', color: resultColor(view.resultKind), marginBottom: '8px' }}>
          {view.result}
        </div>
        <div id="move-project-workspaces">
          {view.workspaces.map(({ workspace_id, workspace_name }) => (
            <button
              key={workspace_id}
              type="button"
              className={
                workspace_id === view.selectedWorkspaceId
                  ? 'move-project-workspace selected'
                  : 'move-project-workspace'
              }
              disabled={view.busy}
              onClick={() => void selectMoveWorkspace(workspace_id)}
            >
              {workspace_name}
            </button>
          ))}
        </div>
        <div
          id="move-project-list"
          style={{ display: 'flex', flexDirection: 'column', gap: '4px', maxHeight: '300px', overflowY: 'auto' }}
        >
          {view.items.map(({ project_id, project_name, description, is_default }) => (
            <button
              key={project_id}
              type="button"
              className="move-project-item"
              disabled={view.busy}
              style={is_default ? { color: '#cf222e', borderColor: '#ffcbc8' } : undefined}
              onClick={() => void doMoveProject(project_id)}
            >
              <span className="move-project-item-name">{project_name}</span>
              {description ? <span className="move-project-item-desc">{description}</span> : null}
            </button>
          ))}
        </div>
        <div id="move-project-dialog-actions">
          <button id="btn-move-project-close" type="button" onClick={() => closeMoveProjectDialog()}>
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}
