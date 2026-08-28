import { useSyncExternalStore } from 'react';
import { closeReadLaterDialog } from '../commands/dialog.ts';
import { readLaterOpenStore } from '../state/dialog-open.ts';
import { ReadLaterList } from './list.tsx';

export { closeReadLaterDialog, openReadLaterDialog } from '../commands/dialog.ts';

export function ReadLaterDialog() {
  const open = useSyncExternalStore(readLaterOpenStore.subscribe, readLaterOpenStore.getSnapshot);

  return (
    <div
      id="read-later-dialog"
      className={open ? 'open' : undefined}
      onClick={(e) => {
        if (e.target === e.currentTarget) closeReadLaterDialog();
      }}
    >
      <div id="read-later-dialog-box">
        <div id="read-later-dialog-header">
          <h3>Read Later</h3>
          <button
            type="button"
            id="btn-read-later-close"
            className="md-header-btn"
            aria-label="Close"
            onClick={() => closeReadLaterDialog()}
          >
            ×
          </button>
        </div>
        <div id="read-later-dialog-body">{open ? <ReadLaterList showTabs initialFilter="unread" /> : null}</div>
      </div>
    </div>
  );
}
