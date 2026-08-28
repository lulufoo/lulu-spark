import { useSyncExternalStore } from 'react';
import {
  closeCommitDialog,
  commitOpenStore,
  doMdCommit,
  doMdRevertAll,
  doMdRevertFile,
  setCommitMessage,
} from '../../commands/viewer/commit.ts';
import { commitViewStore } from '../../state/commit.ts';

export {
  closeCommitDialog,
  hidePendingBadge,
  openCommitDialog,
  showPendingBadge,
} from '../../commands/viewer/commit.ts';
export { doMdCommit, doMdRevertAll, doMdRevertFile };

export function MdCommitDialog() {
  const open = useSyncExternalStore(commitOpenStore.subscribe, commitOpenStore.getSnapshot);
  const view = useSyncExternalStore(commitViewStore.subscribe, commitViewStore.getSnapshot);

  return (
    <div
      id="md-commit-dialog"
      className={open ? 'open' : undefined}
      onClick={(e) => {
        if (e.target === e.currentTarget) closeCommitDialog();
      }}
    >
      <div id="md-commit-dialog-box">
        <div id="md-commit-dialog-title">
          <h3>● Commit changes</h3>
          <button
            id="md-btn-revert-all"
            type="button"
            className={view.revertAllConfirm ? 'confirm' : undefined}
            disabled={view.committing}
            onClick={() => void doMdRevertAll()}
          >
            {view.revertAllConfirm ? 'Revert all changes?' : 'Revert all changes'}
          </button>
        </div>
        <div id="md-commit-file-list">
          {view.loading ? (
            <div style={{ fontSize: 12, color: '#8c959f' }}>Loading…</div>
          ) : view.error ? (
            <div style={{ fontSize: 12, color: '#cf222e' }}>Failed to get status: {view.error}</div>
          ) : view.empty ? (
            <div style={{ fontSize: 13, color: '#8c959f', padding: '4px 0' }}>No changes to commit</div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
              {view.groups.map(({ key, label, files }) => (
                <div className="commit-file-group" key={key}>
                  <div className="commit-file-group-title">
                    {label}（{files.length}）
                  </div>
                  {files.map((f) => (
                    <div className={`commit-file-item ${key}`} key={f}>
                      <span>{f}</span>
                      <button
                        type="button"
                        className="kb-revert-btn"
                        disabled={view.revertingPath === f}
                        onClick={() => void doMdRevertFile(f, key)}
                      >
                        Revert
                      </button>
                    </div>
                  ))}
                </div>
              ))}
            </div>
          )}
        </div>
        <input
          id="md-commit-dialog-msg"
          type="text"
          placeholder="update: edit via viewer"
          value={view.message}
          onChange={(e) => setCommitMessage(e.target.value)}
        />
        <div id="md-commit-dialog-actions">
          <span
            id="md-commit-dialog-result"
            style={
              view.resultKind === 'ok'
                ? { color: '#1a7f37' }
                : view.resultKind === 'err'
                  ? { color: '#cf222e' }
                  : view.resultKind === 'busy'
                    ? { color: '#8c959f' }
                    : undefined
            }
          >
            {view.result}
          </span>
          <button
            id="md-btn-commit-cancel"
            type="button"
            className="md-header-btn"
            onClick={() => closeCommitDialog()}
          >
            Cancel
          </button>
          <button
            id="md-btn-commit-ok"
            type="button"
            className="md-header-btn primary"
            disabled={!view.canCommit || view.committing}
            onClick={() => void doMdCommit()}
          >
            Commit
          </button>
        </div>
      </div>
    </div>
  );
}
