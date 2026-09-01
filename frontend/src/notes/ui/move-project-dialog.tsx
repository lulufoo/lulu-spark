import { useSyncExternalStore } from 'react';
import {
  closeMoveProjectDialog,
  doMoveProject,
  moveProjectOpenStore,
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
        <h3>↷ Switch project</h3>
        <div id="move-project-result" style={{ fontSize: '12px', color: resultColor(view.resultKind), marginBottom: '8px' }}>
          {view.result}
        </div>
        <div
          id="move-project-list"
          style={{ display: 'flex', flexDirection: 'column', gap: '4px', maxHeight: '300px', overflowY: 'auto' }}
        >
          {view.items.map(({ proj, title, desc }) => {
            const inbox = proj === 'inbox';
            return (
              <button
                key={proj}
                type="button"
                className="move-project-item"
                disabled={view.busy}
                style={inbox ? { color: '#cf222e', borderColor: '#ffcbc8' } : undefined}
                onClick={() => void doMoveProject(proj)}
              >
                <span className="move-project-item-name">{title}</span>
                {desc ? <span className="move-project-item-desc">{desc}</span> : null}
              </button>
            );
          })}
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
