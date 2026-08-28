/**
 * Home-entry shell A/B/C state machine.
 * Legal edges: A↔B, B↔C, C→B (closeOverlay), C→B→C (switch entry).
 */

export type FsmMode = 'A' | 'B' | 'C';
export type FsmSnapshot = { mode: 'A' } | { mode: 'B' } | { mode: 'C'; entryId: string };
export type FsmEvent =
  | { type: 'openHub' }
  | { type: 'closeHub' }
  | { type: 'openEntry'; entryId: string }
  | { type: 'closeOverlay' }
  | { type: 'forceA' };
export type FsmTransition = { from: FsmMode; to: FsmMode };
export type FsmDispatchResult = { accepted: boolean; transitions: FsmTransition[] };

export function createHomeEntryFsm(initial?: FsmSnapshot) {
  let mode: FsmMode = initial?.mode ?? 'A';
  let entryId: string | null = initial?.mode === 'C' ? initial.entryId : null;

  function snapshot(): FsmSnapshot {
    if (mode === 'C') {
      return { mode: 'C', entryId: entryId as string };
    }
    return { mode };
  }

  function getState(): FsmMode {
    return mode;
  }

  function dispatch(event: FsmEvent): FsmDispatchResult {
    if (event.type === 'forceA') {
      if (mode === 'A') {
        return { accepted: true, transitions: [] };
      }
      const from = mode;
      mode = 'A';
      entryId = null;
      return { accepted: true, transitions: [{ from, to: 'A' }] };
    }

    if (event.type === 'openHub') {
      if (mode !== 'A') {
        return { accepted: false, transitions: [] };
      }
      mode = 'B';
      return { accepted: true, transitions: [{ from: 'A', to: 'B' }] };
    }

    if (event.type === 'closeHub') {
      if (mode !== 'B') {
        return { accepted: false, transitions: [] };
      }
      mode = 'A';
      return { accepted: true, transitions: [{ from: 'B', to: 'A' }] };
    }

    if (event.type === 'openEntry') {
      if (mode === 'A') {
        return { accepted: false, transitions: [] };
      }
      if (mode === 'B') {
        mode = 'C';
        entryId = event.entryId;
        return { accepted: true, transitions: [{ from: 'B', to: 'C' }] };
      }
      if (mode === 'C') {
        if (event.entryId === entryId) {
          return { accepted: false, transitions: [] };
        }
        const transitions: FsmTransition[] = [
          { from: 'C', to: 'B' },
          { from: 'B', to: 'C' },
        ];
        mode = 'C';
        entryId = event.entryId;
        return { accepted: true, transitions };
      }
      return { accepted: false, transitions: [] };
    }

    if (event.type === 'closeOverlay') {
      if (mode !== 'C') {
        return { accepted: false, transitions: [] };
      }
      mode = 'B';
      entryId = null;
      return { accepted: true, transitions: [{ from: 'C', to: 'B' }] };
    }

    return { accepted: false, transitions: [] };
  }

  return { getState, dispatch, snapshot };
}
