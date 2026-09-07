import { useSyncExternalStore } from 'react';
import {
  closeWorkbenchCommitDialog,
  doWorkbenchCommit,
  setWorkbenchCommitMessage,
  workbenchCommitOpenStore,
} from '../commands/workbench-commit-dialog.ts';
import { workbenchCommitViewStore } from '../state/workbench-commit.ts';

export {
  closeWorkbenchCommitDialog,
  doWorkbenchCommit,
  openWorkbenchCommitDialog,
  setWorkbenchCommitMessage,
} from '../commands/workbench-commit-dialog.ts';

export function WorkbenchCommitDialog() {
  const open = useSyncExternalStore(workbenchCommitOpenStore.subscribe, workbenchCommitOpenStore.getSnapshot);
  const view = useSyncExternalStore(workbenchCommitViewStore.subscribe, workbenchCommitViewStore.getSnapshot);

  return (
    <div
      id="workbench-commit-dialog"
      className={open ? 'open' : undefined}
      onClick={(e) => {
        if (e.target === e.currentTarget) closeWorkbenchCommitDialog();
      }}
    >
      <div id="workbench-commit-dialog-box">
        <h3>↑ Workbench commit</h3>
        <div id="workbench-commit-file-list">
          {view.loading ? (
            <div style={{ fontSize: 12, color: '#8c959f' }}>Loading…</div>
          ) : view.error ? (
            <div style={{ fontSize: 12, color: '#cf222e' }}>Failed to get status: {view.error}</div>
          ) : view.empty ? (
            <div style={{ fontSize: 13, color: '#8c959f', padding: '4px 0' }}>No changes to commit or push</div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
              {view.ahead ? (
                <div className="commit-file-group">
                  <div className="commit-file-group-title" style={{ color: '#0969da' }}>
                    Ready to push ({view.ahead} local commits)
                  </div>
                  <div className="commit-file-item" style={{ background: '#ddf4ff', color: '#0550ae' }}>
                    {view.ahead} local commit(s) not yet pushed
                  </div>
                </div>
              ) : null}
              {view.groups.map(({ key, label, files }) => (
                <div className="commit-file-group" key={key}>
                  <div className="commit-file-group-title">
                    {label}（{files.length}）
                  </div>
                  {files.map((f) => (
                    <div className={`commit-file-item ${key}`} key={f}>
                      {f}
                    </div>
                  ))}
                </div>
              ))}
            </div>
          )}
        </div>
        <input
          id="workbench-commit-msg"
          type="text"
          placeholder="Commit message (empty: chore: update via viewer)"
          autoComplete="off"
          value={view.message}
          onChange={(e) => setWorkbenchCommitMessage(e.target.value)}
        />
        <div id="workbench-commit-dialog-actions">
          <span id="workbench-commit-result"></span>
          <button
            id="btn-workbench-commit-cancel"
            type="button"
            onClick={() => closeWorkbenchCommitDialog()}
          >
            Cancel
          </button>
          <button
            id="btn-workbench-commit-ok"
            type="button"
            disabled={!view.canCommit}
            onClick={() => doWorkbenchCommit()}
          >
            {view.okLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
