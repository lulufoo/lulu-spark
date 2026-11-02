import { useState } from 'react';
import type { SyncProgressEntry } from '../../host/api.ts';
import {
  foldGroup,
  groupRoster,
  rowHover,
  type SyncGroupKey,
} from '../state/sync-roster-view.ts';

/**
 * The documents of a round, in groups: Failed, In progress, Done, Not started. One row is an arrow
 * (up: sent to the cloud, down: brought here) and the name. The name opens the document when this
 * machine has it; otherwise it is grey and does nothing. A long group folds into "N more".
 *
 * Paint only: the list is the host's, as the page was given it.
 */
export function SyncRosterList({
  entries,
  onOpenDoc,
}: {
  entries: readonly SyncProgressEntry[];
  /** The user pressed the name of a document this machine has. */
  onOpenDoc: (id: string) => void;
}) {
  const [opened, setOpened] = useState<ReadonlySet<SyncGroupKey>>(new Set());
  const groups = groupRoster(entries);
  if (groups.length === 0) return null;

  return (
    <div className="home-sync-roster" data-role="sync-roster">
      {groups.map((group) => {
        const { shown, hidden } = foldGroup(group.entries, opened.has(group.key));
        return (
          <section key={group.key} className="home-sync-group" data-role="sync-group" data-group={group.key}>
            <h3 className="home-sync-group-title" data-role="sync-group-title">
              {group.label} · {group.entries.length}
            </h3>
            <ul className="home-sync-rows">
              {shown.map((entry) => (
                <li
                  key={`${entry.op}:${entry.id}`}
                  className="home-sync-row"
                  data-role="sync-row"
                  data-op={entry.op}
                  data-state={entry.state}
                  data-tip={rowHover(entry)}
                >
                  <span
                    className="home-sync-row-arrow"
                    role="img"
                    aria-label={entry.op === 'upload' ? 'Upload' : 'Download'}
                  >
                    {entry.op === 'upload' ? '↑' : '↓'}
                  </span>
                  {entry.local ? (
                    <button
                      type="button"
                      className="home-sync-row-name"
                      data-role="sync-row-name"
                      onClick={() => onOpenDoc(entry.id)}
                    >
                      {entry.name}
                    </button>
                  ) : (
                    <span className="home-sync-row-name" data-role="sync-row-name" data-disabled="true">
                      {entry.name}
                    </span>
                  )}
                </li>
              ))}
            </ul>
            {hidden > 0 ? (
              <button
                type="button"
                className="home-sync-more"
                data-role="sync-more"
                onClick={() => setOpened((prev) => new Set(prev).add(group.key))}
              >
                {hidden} more
              </button>
            ) : null}
          </section>
        );
      })}
    </div>
  );
}
