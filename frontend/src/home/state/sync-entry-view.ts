import { isBusy } from './sync-entry-flow.ts';
import type { SyncCheck, SyncRun, SyncSeen } from './sync-entry.ts';

/** What the entry shows. Nothing of it is stored: it is worked out from the states below. */
export type SyncEntryView = {
  /** The dot on the icon in the account row; `pulse` fades in and out while a round goes on. */
  icon: 'none' | 'dot' | 'pulse';
  start: {
    /** There is something to sync. */
    dot: boolean;
    /** A round is going on: the item turns and cannot be pressed. */
    spinning: boolean;
    enabled: boolean;
  };
  view: {
    /** The round that ended last has not been opened. */
    dot: boolean;
    /** A round is going on, or one ended since the app started (even one that could not start). */
    enabled: boolean;
  };
};

/**
 * The seven states of the entry, from three of the four stores (the list is for the panel).
 *
 * | # | run | check | seen | icon | Start | View |
 * |---|---|---|---|---|---|---|
 * | 1 | idle | none | – | none | pressable | grey |
 * | 2 | idle | pending | – | dot | dot | grey |
 * | 3 | going on | – | – | pulse | turning, grey | pressable |
 * | 4 | ended | none | unseen | dot | no dot | dot |
 * | 5 | ended | pending | unseen | dot | dot | dot |
 * | 6 | ended | pending | seen | dot | dot | no dot |
 * | 7 | ended | none | seen | none | no dot | no dot |
 *
 * A check that is under way or was never made shows no dot, like `none`. State 5 is state 4 with
 * documents that failed: they are still behind, so the check finds them again.
 */
export function deriveSyncEntry(run: SyncRun, check: SyncCheck, seen: SyncSeen): SyncEntryView {
  if (isBusy(run)) {
    return {
      icon: 'pulse',
      start: { dot: false, spinning: true, enabled: false },
      view: { dot: false, enabled: true },
    };
  }
  const startDot = check.phase === 'pending';
  const hasResult = run.phase === 'ended';
  const viewDot = hasResult && seen === 'unseen';
  return {
    icon: startDot || viewDot ? 'dot' : 'none',
    start: { dot: startDot, spinning: false, enabled: true },
    view: { dot: viewDot, enabled: hasResult },
  };
}
