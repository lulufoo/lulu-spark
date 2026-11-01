import { useState } from 'react';
import type { CloudSyncView } from '../state/cloud-sync.ts';
import { cloudSyncStatus } from './cloud-sync-text.ts';
import { HomeNavIcon } from './nav-icons.tsx';

/**
 * The sync icon beside the `?` of the account row. Pressing it starts a sync and shows what the
 * sync is doing. While one goes on the icon keeps turning; pressing it then only shows the status
 * again. The panel holds the status, and a Cancel while the sync goes on.
 *
 * Paint only: what a sync is, and what cancelling does, is decided by the page's commands.
 */
export function CloudSyncButton({
  sync,
  onSync,
  onCancel,
}: {
  sync: CloudSyncView;
  /** Asks for a sync. */
  onSync: () => void;
  /** Asks the sync that is going on to stop. */
  onCancel: () => void;
}) {
  const [open, setOpen] = useState(false);
  const syncing = sync.phase === 'syncing';
  const cancelling = sync.phase === 'syncing' && sync.cancelling;
  const status = cloudSyncStatus(sync);
  const label = syncing ? 'Syncing…' : 'Sync';

  function press() {
    if (!syncing) callSafely(onSync);
    setOpen(true);
  }

  return (
    <div className="home-sync-button" data-role="sync-button">
      <button
        type="button"
        className="home-sync-button-trigger"
        data-role="sync-trigger"
        data-state={syncing ? 'syncing' : 'idle'}
        data-tone={!syncing && status ? status.tone : undefined}
        aria-haspopup="dialog"
        aria-expanded={open ? 'true' : 'false'}
        aria-busy={syncing ? 'true' : 'false'}
        aria-label={label}
        data-tip={label}
        onClick={press}
      >
        <HomeNavIcon name="sync" />
      </button>
      {open && status ? (
        <>
          <div
            className="home-sync-button-backdrop"
            data-role="close-sync-panel"
            onClick={() => setOpen(false)}
          />
          <div className="home-sync-button-panel" role="dialog" aria-label="Sync status">
            <div
              className="home-sync-button-status"
              data-role="sync-status"
              data-tone={status.tone}
              role="status"
            >
              <div className="home-sync-button-title" data-role="sync-status-title">
                {status.title}
              </div>
              {status.detail ? (
                <div className="home-sync-button-detail" data-role="sync-status-detail">
                  {status.detail}
                </div>
              ) : null}
            </div>
            {syncing ? (
              <button
                type="button"
                className="home-sync-button-cancel"
                data-role="sync-cancel"
                disabled={cancelling}
                onClick={() => callSafely(onCancel)}
              >
                {cancelling ? 'Cancelling…' : 'Cancel'}
              </button>
            ) : null}
          </div>
        </>
      ) : null}
    </div>
  );
}

function callSafely(fn: () => void) {
  try {
    fn();
  } catch {
    /* keep the button clickable */
  }
}
