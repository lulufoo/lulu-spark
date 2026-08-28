import { useSyncExternalStore } from 'react';
import {
  closeCommitChangesDialog,
  commitChangesOpenStore,
  doCommitChanges,
  setCommitChangesMessage,
} from '../commands/commit-dialog.ts';
import { commitChangesViewStore } from '../state/commit-changes.ts';

export {
  closeCommitChangesDialog,
  doCommitChanges,
  openCommitChangesDialog,
  setCommitChangesMessage,
} from '../commands/commit-dialog.ts';

export function CommitChangesDialog() {
  const open = useSyncExternalStore(commitChangesOpenStore.subscribe, commitChangesOpenStore.getSnapshot);
  const view = useSyncExternalStore(commitChangesViewStore.subscribe, commitChangesViewStore.getSnapshot);

  return (
    <div
      id="commit-changes-dialog"
      className={open ? 'open' : undefined}
      onClick={(e) => {
        if (e.target === e.currentTarget) closeCommitChangesDialog();
      }}
    >
      <div id="commit-changes-dialog-box">
        <h3>↑ Commit changes</h3>
        <div id="commit-changes-file-list">
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
          id="commit-changes-msg"
          type="text"
          placeholder="Commit message (empty: chore: update via viewer)"
          autoComplete="off"
          value={view.message}
          onChange={(e) => setCommitChangesMessage(e.target.value)}
        />
        <div id="commit-changes-dialog-actions">
          <span id="commit-changes-result"></span>
          <button
            id="btn-commit-changes-cancel"
            type="button"
            onClick={() => closeCommitChangesDialog()}
          >
            Cancel
          </button>
          <button
            id="btn-commit-changes-ok"
            type="button"
            disabled={!view.canCommit}
            onClick={() => doCommitChanges()}
          >
            {view.okLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
