import { useSyncExternalStore } from 'react';
import {
  closeSparkCommitDialog,
  doSparkCommit,
  setSparkCommitMessage,
  sparkCommitOpenStore,
} from '../commands/spark-commit-dialog.ts';
import { sparkCommitViewStore } from '../state/spark-commit.ts';

export {
  closeSparkCommitDialog,
  doSparkCommit,
  openSparkCommitDialog,
  setSparkCommitMessage,
} from '../commands/spark-commit-dialog.ts';

export function SparkCommitDialog() {
  const open = useSyncExternalStore(sparkCommitOpenStore.subscribe, sparkCommitOpenStore.getSnapshot);
  const view = useSyncExternalStore(sparkCommitViewStore.subscribe, sparkCommitViewStore.getSnapshot);

  return (
    <div
      id="spark-commit-dialog"
      className={open ? 'open' : undefined}
      onClick={(e) => {
        if (e.target === e.currentTarget) closeSparkCommitDialog();
      }}
    >
      <div id="spark-commit-dialog-box">
        <h3>↑ Spark commit</h3>
        <div id="spark-commit-file-list">
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
          id="spark-commit-msg"
          type="text"
          placeholder="Commit message (empty: chore: update via viewer)"
          autoComplete="off"
          value={view.message}
          onChange={(e) => setSparkCommitMessage(e.target.value)}
        />
        <div id="spark-commit-dialog-actions">
          <span id="spark-commit-result"></span>
          <button
            id="btn-spark-commit-cancel"
            type="button"
            onClick={() => closeSparkCommitDialog()}
          >
            Cancel
          </button>
          <button
            id="btn-spark-commit-ok"
            type="button"
            disabled={!view.canCommit}
            onClick={() => doSparkCommit()}
          >
            {view.okLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
