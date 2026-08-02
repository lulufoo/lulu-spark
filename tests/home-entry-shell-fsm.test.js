import { describe, it, expect, beforeEach } from 'vitest';

import { createHomeEntryFsm } from '../frontend/js/home-entry-shell/fsm.js';

/**
 * Table-driven A/B/C FSM for the home-entry shell (T2).
 * Events: openHub / closeHub / openEntry(entryId) / closeOverlay / forceA
 */

describe('home-entry-shell fsm · A/B/C legal edges + snapshot (T2)', () => {
  /** @type {ReturnType<typeof createHomeEntryFsm>} */
  let fsm;

  beforeEach(() => {
    fsm = createHomeEntryFsm();
  });

  it('defaults to collapsed A; snapshot exposes mode (no entryId)', () => {
    expect(fsm.getState()).toBe('A');
    expect(fsm.snapshot()).toEqual({ mode: 'A' });
  });

  it('legal A↔B via openHub / closeHub', () => {
    const toB = fsm.dispatch({ type: 'openHub' });
    expect(toB.accepted).toBe(true);
    expect(fsm.getState()).toBe('B');
    expect(fsm.snapshot()).toEqual({ mode: 'B' });

    const toA = fsm.dispatch({ type: 'closeHub' });
    expect(toA.accepted).toBe(true);
    expect(fsm.getState()).toBe('A');
    expect(fsm.snapshot()).toEqual({ mode: 'A' });
  });

  it('legal B↔C via openEntry / closeOverlay; C snapshot carries entryId', () => {
    fsm.dispatch({ type: 'openHub' });

    const toC = fsm.dispatch({ type: 'openEntry', entryId: 'read-later' });
    expect(toC.accepted).toBe(true);
    expect(fsm.getState()).toBe('C');
    expect(fsm.snapshot()).toEqual({ mode: 'C', entryId: 'read-later' });

    const toB = fsm.dispatch({ type: 'closeOverlay' });
    expect(toB.accepted).toBe(true);
    expect(fsm.getState()).toBe('B');
    expect(fsm.snapshot()).toEqual({ mode: 'B' });
  });

  it('switching business from C must go C→B→C (no C→C direct edge)', () => {
    fsm.dispatch({ type: 'openHub' });
    fsm.dispatch({ type: 'openEntry', entryId: 'read-later' });
    expect(fsm.snapshot()).toEqual({ mode: 'C', entryId: 'read-later' });

    const switched = fsm.dispatch({ type: 'openEntry', entryId: 'plan-task' });
    expect(switched.accepted).toBe(true);
    // Composite path must include the intermediate B — never a single C→C hop.
    expect(switched.transitions).toEqual([
      { from: 'C', to: 'B' },
      { from: 'B', to: 'C' },
    ]);
    expect(fsm.getState()).toBe('C');
    expect(fsm.snapshot()).toEqual({ mode: 'C', entryId: 'plan-task' });
  });

  it('closeHub while in C keeps C (no transition to A)', () => {
    fsm.dispatch({ type: 'openHub' });
    fsm.dispatch({ type: 'openEntry', entryId: 'notes' });

    const result = fsm.dispatch({ type: 'closeHub' });
    expect(result.accepted).toBe(false);
    expect(result.transitions).toEqual([]);
    expect(fsm.getState()).toBe('C');
    expect(fsm.snapshot()).toEqual({ mode: 'C', entryId: 'notes' });
  });

  it('rejects illegal A→C (openEntry from A)', () => {
    const result = fsm.dispatch({ type: 'openEntry', entryId: 'builders' });
    expect(result.accepted).toBe(false);
    expect(result.transitions).toEqual([]);
    expect(fsm.getState()).toBe('A');
    expect(fsm.snapshot()).toEqual({ mode: 'A' });
  });

  it('rejects illegal C→A via closeOverlay chain; must C→B then B→A', () => {
    fsm.dispatch({ type: 'openHub' });
    fsm.dispatch({ type: 'openEntry', entryId: 'builders' });

    // closeOverlay only reaches B, never A
    const close = fsm.dispatch({ type: 'closeOverlay' });
    expect(close.accepted).toBe(true);
    expect(close.transitions).toEqual([{ from: 'C', to: 'B' }]);
    expect(fsm.getState()).toBe('B');

    const collapse = fsm.dispatch({ type: 'closeHub' });
    expect(collapse.accepted).toBe(true);
    expect(collapse.transitions).toEqual([{ from: 'B', to: 'A' }]);
    expect(fsm.getState()).toBe('A');
  });

  it('forceA recovers to A from any mode (including C)', () => {
    fsm.dispatch({ type: 'openHub' });
    fsm.dispatch({ type: 'openEntry', entryId: 'notes' });
    expect(fsm.getState()).toBe('C');

    const forced = fsm.dispatch({ type: 'forceA' });
    expect(forced.accepted).toBe(true);
    expect(fsm.getState()).toBe('A');
    expect(fsm.snapshot()).toEqual({ mode: 'A' });
  });

  it('same entryId openEntry while in C is rejected (no C→C)', () => {
    fsm.dispatch({ type: 'openHub' });
    fsm.dispatch({ type: 'openEntry', entryId: 'read-later' });

    const same = fsm.dispatch({ type: 'openEntry', entryId: 'read-later' });
    expect(same.accepted).toBe(false);
    expect(same.transitions).toEqual([]);
    expect(fsm.snapshot()).toEqual({ mode: 'C', entryId: 'read-later' });
  });

  it('accepts optional initial state', () => {
    const fromB = createHomeEntryFsm({ mode: 'B' });
    expect(fromB.snapshot()).toEqual({ mode: 'B' });

    const fromC = createHomeEntryFsm({ mode: 'C', entryId: 'notes' });
    expect(fromC.snapshot()).toEqual({ mode: 'C', entryId: 'notes' });
  });

  // T7 hard counterexamples — FSM surface (business A↔C still illegal; AI bypass is the exception)
  it('hard counterexample: business A→C still illegal; business C has no closeHub→A', () => {
    const fsm = createHomeEntryFsm();
    expect(fsm.dispatch({ type: 'openEntry', entryId: 'notes' }).accepted).toBe(false);
    expect(fsm.getState()).toBe('A');

    fsm.dispatch({ type: 'openHub' });
    fsm.dispatch({ type: 'openEntry', entryId: 'notes' });
    expect(fsm.dispatch({ type: 'closeHub' }).accepted).toBe(false);
    expect(fsm.getState()).toBe('C');
    // Business closeOverlay lands on B, never A.
    expect(fsm.dispatch({ type: 'closeOverlay' }).transitions).toEqual([{ from: 'C', to: 'B' }]);
    expect(fsm.getState()).toBe('B');
  });
});

/**
 * SK-1 / T1 — AI bypass FSM specialization (A→C_AI / C_AI→A).
 * AI entry id is locked to `ai-assistant`.
 */
describe('home-entry-shell fsm · AI bypass A→C_AI / C_AI→A (T1)', () => {
  const AI = 'ai-assistant';

  /** @type {ReturnType<typeof createHomeEntryFsm>} */
  let fsm;

  beforeEach(() => {
    fsm = createHomeEntryFsm();
  });

  it('A→C_AI: openEntry(ai-assistant) from A is accepted', () => {
    const result = fsm.dispatch({ type: 'openEntry', entryId: AI });
    expect(result.accepted).toBe(true);
    expect(result.transitions).toEqual([{ from: 'A', to: 'C' }]);
    expect(fsm.getState()).toBe('C');
    expect(fsm.snapshot()).toEqual({ mode: 'C', entryId: AI });
  });

  it('C_AI→A: closeOverlay from AI C returns A (not B)', () => {
    fsm.dispatch({ type: 'openEntry', entryId: AI });
    const close = fsm.dispatch({ type: 'closeOverlay' });
    expect(close.accepted).toBe(true);
    expect(close.transitions).toEqual([{ from: 'C', to: 'A' }]);
    expect(fsm.getState()).toBe('A');
    expect(fsm.snapshot()).toEqual({ mode: 'A' });
  });

  it('hub business openEntry remains A-illegal; business closeOverlay still → B', () => {
    expect(fsm.dispatch({ type: 'openEntry', entryId: 'builders' }).accepted).toBe(false);
    expect(fsm.getState()).toBe('A');

    fsm.dispatch({ type: 'openHub' });
    fsm.dispatch({ type: 'openEntry', entryId: 'builders' });
    const close = fsm.dispatch({ type: 'closeOverlay' });
    expect(close.accepted).toBe(true);
    expect(close.transitions).toEqual([{ from: 'C', to: 'B' }]);
    expect(fsm.getState()).toBe('B');
  });

  it('rejects silent A→B→C for AI: no composite open that fabricates hub path from A', () => {
    // From A, only the AI specialization edge is legal — not a fabricated hub expand.
    const biz = fsm.dispatch({ type: 'openEntry', entryId: 'notes' });
    expect(biz.accepted).toBe(false);
    expect(fsm.getState()).toBe('A');
    // openHub alone is A→B; AI does not require or invent A→B→C.
    const hub = fsm.dispatch({ type: 'openHub' });
    expect(hub.accepted).toBe(true);
    expect(hub.transitions).toEqual([{ from: 'A', to: 'B' }]);
  });

  it('idempotent: openEntry(ai-assistant) while already C_AI is rejected (no C→C)', () => {
    fsm.dispatch({ type: 'openEntry', entryId: AI });
    const again = fsm.dispatch({ type: 'openEntry', entryId: AI });
    expect(again.accepted).toBe(false);
    expect(again.transitions).toEqual([]);
    expect(fsm.snapshot()).toEqual({ mode: 'C', entryId: AI });
  });
});
