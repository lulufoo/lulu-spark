// @vitest-environment jsdom
/**
 * T7 — interactive/state verification suite (VF).
 * Consolidates legal edges, dual B→A, multi C→B, force-A recovery,
 * hard counterexamples, and non-blocking motion against the shell model.
 */
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { createHomeEntryFsm } from '../../frontend/js/home-entry-shell/fsm.js';
import { readMainSource } from '../helpers/read-frontend-js.js';
import {
  mountShellIntegrationFixture,
  hub,
  entryBtn,
  closeBtn,
  backdrop,
  overlay,
  contentSlot,
} from '../fixtures/home-entry-shell-integration.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function readMain() {
  return readMainSource();
}

describe('home-entry-shell integration · legal edges + triggers + hard counterexamples (T7)', () => {
  /** @type {ReturnType<typeof mountShellIntegrationFixture> | null} */
  let fx = null;

  beforeEach(() => {
    document.body.innerHTML = '';
  });

  afterEach(() => {
    fx?.shell.unmount();
    fx = null;
    document.body.innerHTML = '';
  });

  it('legal edges reachable: A↔B and B↔C; C snapshot carries entryId', async () => {
    fx = mountShellIntegrationFixture();
    const { anchor, shell } = fx;

    expect(shell.getState()).toEqual({ mode: 'A' });

    hub(anchor).click();
    expect(shell.getState()).toEqual({ mode: 'B' });

    await shell.openContent('read-later');
    expect(shell.getState()).toEqual({ mode: 'C', entryId: 'read-later' });
    expect(overlay(anchor).hidden).toBe(false);
    expect(contentSlot(anchor).querySelector('.shell-content-marker')?.dataset.entryId).toBe(
      'read-later',
    );

    closeBtn(anchor).click();
    expect(shell.getState()).toEqual({ mode: 'B' });

    hub(anchor).click();
    expect(shell.getState()).toEqual({ mode: 'A' });
  });

  it('illegal A↔C rejected at FSM; shell cannot open content from A', async () => {
    const fsm = createHomeEntryFsm();
    expect(fsm.dispatch({ type: 'openEntry', entryId: 'notes' }).accepted).toBe(false);
    expect(fsm.getState()).toBe('A');

    fx = mountShellIntegrationFixture();
    const { shell } = fx;
    await shell.openContent('notes');
    expect(shell.getState()).toEqual({ mode: 'A' });
  });

  it('switching business from C goes C→B→C (no C→C); hard counterexample #2 fails if skipped', async () => {
    fx = mountShellIntegrationFixture();
    const { anchor, shell } = fx;

    hub(anchor).click();
    await shell.openContent('read-later');
    expect(shell.getState()).toEqual({ mode: 'C', entryId: 'read-later' });

    // UI path: click another entry while overlay is open.
    entryBtn(anchor, 'notes').click();
    await Promise.resolve();
    await Promise.resolve();
    expect(shell.getState()).toEqual({ mode: 'C', entryId: 'notes' });
    expect(contentSlot(anchor).querySelector('.shell-content-marker')?.dataset.entryId).toBe(
      'notes',
    );

    // Pure FSM path must expose the intermediate B hop.
    const fsm = createHomeEntryFsm({ mode: 'C', entryId: 'read-later' });
    const switched = fsm.dispatch({ type: 'openEntry', entryId: 'todo-task' });
    expect(switched.accepted).toBe(true);
    expect(switched.transitions).toEqual([
      { from: 'C', to: 'B' },
      { from: 'B', to: 'C' },
    ]);
  });

  it('B→A dual triggers: outside cluster AND re-click hub (hard counterexample #4 if either missing)', () => {
    fx = mountShellIntegrationFixture();
    const { anchor, shell } = fx;

    hub(anchor).click();
    expect(shell.getState()).toEqual({ mode: 'B' });
    document.body.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(shell.getState()).toEqual({ mode: 'A' });

    hub(anchor).click();
    expect(shell.getState()).toEqual({ mode: 'B' });
    hub(anchor).click();
    expect(shell.getState()).toEqual({ mode: 'A' });
  });

  it('C→B multi-triggers: close / backdrop·outside / Escape all land on B (hard counterexample #5)', async () => {
    fx = mountShellIntegrationFixture();
    const { anchor, shell } = fx;

    hub(anchor).click();
    await shell.openContent('notes');
    closeBtn(anchor).click();
    expect(shell.getState()).toEqual({ mode: 'B' });

    await shell.openContent('todo-task');
    backdrop(anchor).click();
    expect(shell.getState()).toEqual({ mode: 'B' });

    await shell.openContent('builders');
    document.body.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(shell.getState()).toEqual({ mode: 'B' });

    await shell.openContent('read-later');
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    expect(shell.getState()).toEqual({ mode: 'B' });
  });

  it('C: hub click stays in C (hard counterexample #6 — must not jump to A)', async () => {
    fx = mountShellIntegrationFixture();
    const { anchor, shell } = fx;

    hub(anchor).click();
    await shell.openContent('builders');
    expect(shell.getState()).toEqual({ mode: 'C', entryId: 'builders' });

    hub(anchor).click();
    expect(shell.getState()).toEqual({ mode: 'C', entryId: 'builders' });
    expect(overlay(anchor).hidden).toBe(false);
  });

  it('exception / close-failure / leave-unmount all force A', async () => {
    fx = mountShellIntegrationFixture();
    const { anchor, shell } = fx;

    hub(anchor).click();
    await shell.openContent('read-later');
    shell.forceRecoverA('exception');
    expect(shell.getState()).toEqual({ mode: 'A' });
    expect(overlay(anchor).hidden).toBe(true);

    // close-failure fixture: unmount throws → force A
    fx.shell.unmount();
    fx = mountShellIntegrationFixture({ closeUnmountThrows: true });
    const fx2 = fx;
    hub(fx2.anchor).click();
    await fx2.shell.openContent('throw-close');
    expect(fx2.shell.getState().mode).toBe('C');
    closeBtn(fx2.anchor).click();
    await Promise.resolve();
    await Promise.resolve();
    expect(fx2.shell.getState()).toEqual({ mode: 'A' });

    // leave / host teardown
    fx2.shell.unmount();
    fx = mountShellIntegrationFixture();
    hub(fx.anchor).click();
    await fx.shell.openContent('notes');
    expect(fx.shell.getState().mode).toBe('C');
    fx.shell.unmount();
    expect(fx.shell.getState()).toEqual({ mode: 'A' });
    fx = null;
  });

  it('motion is non-blocking: state flips without waiting for animationend/transitionend', async () => {
    fx = mountShellIntegrationFixture();
    const { anchor, shell } = fx;
    const animEnd = vi.fn();
    anchor.addEventListener('animationend', animEnd);
    anchor.addEventListener('transitionend', animEnd);

    hub(anchor).click();
    expect(shell.getState()).toEqual({ mode: 'B' });

    await shell.openContent('notes');
    expect(shell.getState()).toEqual({ mode: 'C', entryId: 'notes' });

    closeBtn(anchor).click();
    expect(shell.getState()).toEqual({ mode: 'B' });

    hub(anchor).click();
    expect(shell.getState()).toEqual({ mode: 'A' });

    expect(animEnd).not.toHaveBeenCalled();
  });

  it('hard counterexample #1/#3: no A↔C path; orchestration is a single shell mount', () => {
    const fsm = createHomeEntryFsm();
    fsm.dispatch({ type: 'openHub' });
    fsm.dispatch({ type: 'openEntry', entryId: 'notes' });
    // closeOverlay must not reach A
    expect(fsm.dispatch({ type: 'closeOverlay' }).transitions).toEqual([
      { from: 'C', to: 'B' },
    ]);
    expect(fsm.getState()).toBe('B');

    // openEntry from A rejected
    const fromA = createHomeEntryFsm();
    expect(fromA.dispatch({ type: 'openEntry', entryId: 'notes' }).accepted).toBe(false);

    const main = readMain();
    const mounts = main.match(/mountHomeEntryShell\s*\(\s*document\.body\b/g) || [];
    expect(mounts.length).toBe(1);
    expect(main).not.toMatch(
      /mount(?:ReadLater|TodoTask|Note|Builders)AssistantWidget\s*\(\s*document\.body\b/,
    );
  });
});
