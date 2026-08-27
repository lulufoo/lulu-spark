/**
 * Home-entry shell A/B/C state machine.
 * Legal edges: A↔B, B↔C, C→B (closeOverlay), C→B→C (switch entry).
 */

/**
 * @typedef {'A' | 'B' | 'C'} FsmMode
 * @typedef {{ mode: 'A' } | { mode: 'B' } | { mode: 'C', entryId: string }} FsmSnapshot
 * @typedef {{ type: 'openHub' } | { type: 'closeHub' } | { type: 'openEntry', entryId: string } | { type: 'closeOverlay' } | { type: 'forceA' }} FsmEvent
 * @typedef {{ from: FsmMode, to: FsmMode }} FsmTransition
 * @typedef {{ accepted: boolean, transitions: FsmTransition[] }} FsmDispatchResult
 */

/**
 * @param {FsmSnapshot} [initial]
 * @returns {{ getState: () => FsmMode, dispatch: (event: FsmEvent) => FsmDispatchResult, snapshot: () => FsmSnapshot }}
 */
export function createHomeEntryFsm(initial) {
  /** @type {FsmMode} */
  let mode = initial?.mode ?? 'A';
  /** @type {string | null} */
  let entryId = initial?.mode === 'C' ? initial.entryId : null;

  function snapshot() {
    if (mode === 'C') {
      return { mode: 'C', entryId: /** @type {string} */ (entryId) };
    }
    return { mode };
  }

  function getState() {
    return mode;
  }

  /**
   * @param {FsmEvent} event
   * @returns {FsmDispatchResult}
   */
  function dispatch(event) {
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
        const transitions = [
          { from: /** @type {FsmMode} */ ('C'), to: /** @type {FsmMode} */ ('B') },
          { from: /** @type {FsmMode} */ ('B'), to: /** @type {FsmMode} */ ('C') },
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
