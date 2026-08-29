import { useSyncExternalStore } from 'react';
import {
  REVERTABLE,
  closeKbCommitDialog,
  doKbCommit,
  kbRevertAll,
  kbRevertFile,
  setKbCommitMessage,
} from '../../commands/viewer/commit.ts';
import { kbCommitOpenStore } from '../../state/dialog-open.ts';
import { kbCommitViewStore } from '../../state/commit.ts';

export {
  closeKbCommitDialog,
  doKbCommit,
  openKbCommitDialog,
} from '../../commands/viewer/commit.ts';
export { kbRevertAll as _kbRevertAll } from '../../commands/viewer/commit.ts';

export function KbCommitDialog() {
  const open = useSyncExternalStore(kbCommitOpenStore.subscribe, kbCommitOpenStore.getSnapshot);
  const view = useSyncExternalStore(kbCommitViewStore.subscribe, kbCommitViewStore.getSnapshot);

  return (
    <div
      id="kb-commit-dialog"
      className={open ? 'open' : undefined}
      onClick={(e) => {
        if (e.target === e.currentTarget) closeKbCommitDialog();
      }}
    >
      <div id="kb-commit-dialog-box">
        <div id="kb-commit-dialog-title">
          <h3>● Commit changes</h3>
          <button
            id="kb-btn-revert-all"
            type="button"
            className={view.revertAllConfirm ? 'confirm' : undefined}
            disabled={view.revertAllDisabled || view.committing}
            onClick={() => {
              void kbRevertAll();
            }}
          >
            {view.revertAllConfirm ? '⚠ Revert all?' : 'Revert all changes'}
          </button>
        </div>
        <div id="kb-commit-file-list">
          {view.loading ? (
            <div style={{ fontSize: 12, color: '#8c959f' }}>Loading…</div>
          ) : view.error ? (
            <div style={{ fontSize: 12, color: '#cf222e' }}>Failed to get status: {view.error}</div>
          ) : view.empty ? (
            <div style={{ fontSize: 13, color: '#8c959f', padding: '4px 0' }}>No changes to commit</div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
              {view.ahead ? (
                <div className="commit-file-group">
                  <div className="commit-file-group-title" style={{ color: '#0969da' }}>
                    Ready to push ({view.ahead} local commits)
                  </div>
                  <div
                    className="commit-file-item"
                    style={{ background: '#ddf4ff', color: '#0550ae', display: 'block' }}
                  >
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
                      <span className="commit-file-item-path">{f}</span>
                      {REVERTABLE.has(key) ? (
                        <button
                          type="button"
                          className="kb-revert-btn"
                          title={`Revert changes to ${f}`}
                          disabled={view.revertingPath === f}
                          onClick={() => void kbRevertFile(f, key)}
                        >
                          Revert
                        </button>
                      ) : null}
                    </div>
                  ))}
                </div>
              ))}
            </div>
          )}
        </div>
        <input
          id="kb-commit-msg"
          type="text"
          placeholder="chore: update via viewer"
          value={view.message}
          onChange={(e) => setKbCommitMessage(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault();
              void doKbCommit();
            }
            if (e.key === 'Escape') {
              e.stopPropagation();
              closeKbCommitDialog();
            }
          }}
        />
        <div id="kb-commit-dialog-actions">
          <span
            id="kb-commit-result"
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
            id="kb-btn-commit-cancel"
            type="button"
            className="md-header-btn"
            onClick={() => closeKbCommitDialog()}
          >
            Cancel
          </button>
          <button
            id="kb-btn-commit-ok"
            type="button"
            className="md-header-btn primary"
            disabled={!view.canCommit || view.committing}
            onClick={() => void doKbCommit()}
          >
            {view.okLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
