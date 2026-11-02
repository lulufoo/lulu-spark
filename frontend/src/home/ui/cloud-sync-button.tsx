import { useEffect, useState } from 'react';
import type { SyncEntryView } from '../state/sync-entry-view.ts';
import type { SyncRoster, SyncRun } from '../state/sync-entry.ts';
import { hasFailedDocs } from '../state/sync-roster-view.ts';
import { HomeNavIcon } from './nav-icons.tsx';
import { syncCardText } from './sync-entry-text.ts';
import { SyncRosterList } from './sync-roster.tsx';

type Open = 'closed' | 'menu' | 'view';

/**
 * The sync icon beside the `?` of the account row. Pressing it opens a menu with Start and View;
 * it never starts a sync by itself. Start begins a round of sync. View opens the panel with the
 * state of the round that is going on, or of the last one: a line that says how it went, the
 * documents of the round in groups, and Cancel while one goes on.
 *
 * Red dots mark what wants attention: on the icon (it fades in and out while a round goes on), on
 * Start (something is behind) and on View (a result nobody has opened). Start turns while a round
 * goes on, and cannot be pressed. Which of these shows is `entry`; this component only paints it.
 *
 * Paint only: what a round is, and what cancelling does, is decided by the page's commands.
 */
export function CloudSyncButton({
  entry,
  run,
  roster,
  onStart,
  onCancel,
  onViewOpen,
  onOpenDoc,
}: {
  entry: SyncEntryView;
  /** For the panel: where the round is, and how it ended. */
  run: SyncRun;
  /** For the panel: the documents of the round, as the host listed them last. */
  roster: SyncRoster;
  /** Asks for a round of sync. */
  onStart: () => void;
  /** Asks the round that is going on to stop. */
  onCancel: () => void;
  /** The panel is open, and shows the result of the round: the user has seen it. */
  onViewOpen: () => void;
  /** The user pressed the name of a document this machine has; the panel closes first. */
  onOpenDoc: (id: string) => void;
}) {
  const [open, setOpen] = useState<Open>('closed');
  // There is nothing to view any more (the user signed out): the card is closed, not just hidden.
  if (open === 'view' && !entry.view.enabled) setOpen('closed');
  const card = open === 'view' && entry.view.enabled ? syncCardText(run, hasFailedDocs(roster.entries)) : null;
  const busy = entry.start.spinning;
  const cancelling = run.phase === 'cancelling';

  // A round that ends while its card is open is shown the moment it ends: that counts as seen.
  useEffect(() => {
    if (card && run.phase === 'ended') callSafely(onViewOpen);
    // `onViewOpen` is the page's and may be new at each render; it is the round that moves this.
  }, [run, open]);

  function choose(next: Open, action: () => void) {
    callSafely(action);
    setOpen(next);
  }

  function openDoc(id: string) {
    setOpen('closed');
    callSafely(() => onOpenDoc(id));
  }

  return (
    <div className="home-sync-button" data-role="sync-button">
      <div className="home-sync-button-anchor">
        <button
          type="button"
          className="home-sync-button-trigger"
          data-role="sync-trigger"
          data-dot={entry.icon}
          aria-haspopup="menu"
          aria-expanded={open === 'menu' ? 'true' : 'false'}
          aria-busy={busy ? 'true' : 'false'}
          aria-label="Sync"
          data-tip="Sync"
          onClick={() => setOpen((prev) => (prev === 'closed' ? 'menu' : 'closed'))}
        >
          <HomeNavIcon name="sync" />
        </button>
        {open === 'menu' ? (
          <>
            <div
              className="home-sync-button-backdrop"
              data-role="close-sync-menu"
              onClick={() => setOpen('closed')}
            />
            <div className="home-sync-menu" role="menu" data-role="sync-menu">
              <button
                type="button"
                role="menuitem"
                className="home-sync-menu-item"
                data-role="sync-start"
                disabled={!entry.start.enabled}
                onClick={() => choose('closed', onStart)}
              >
                <span className="home-sync-menu-label">Start</span>
                {busy ? <span className="home-sync-spinner" data-role="sync-start-spinner" aria-hidden="true" /> : null}
                {entry.start.dot ? <span className="home-sync-dot" data-role="sync-start-dot" aria-hidden="true" /> : null}
              </button>
              <button
                type="button"
                role="menuitem"
                className="home-sync-menu-item"
                data-role="sync-view"
                disabled={!entry.view.enabled}
                onClick={() => choose('view', onViewOpen)}
              >
                <span className="home-sync-menu-label">View</span>
                {entry.view.dot ? <span className="home-sync-dot" data-role="sync-view-dot" aria-hidden="true" /> : null}
              </button>
            </div>
          </>
        ) : null}
      </div>
      {card ? (
        <>
          <div
            className="home-sync-button-backdrop"
            data-role="close-sync-panel"
            onClick={() => setOpen('closed')}
          />
          <div className="home-sync-button-panel" role="dialog" aria-label="Sync status" data-role="sync-panel">
            <div
              className="home-sync-button-status"
              data-role="sync-status"
              data-tone={card.tone}
              role="status"
            >
              <div className="home-sync-button-title" data-role="sync-status-title">
                {card.title}
              </div>
              {card.detail ? (
                <div className="home-sync-button-detail" data-role="sync-status-detail">
                  {card.detail}
                </div>
              ) : null}
            </div>
            <SyncRosterList entries={roster.entries} onOpenDoc={openDoc} />
            {busy ? (
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
