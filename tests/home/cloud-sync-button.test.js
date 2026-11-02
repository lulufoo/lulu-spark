// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../../frontend/src/host/api.ts';
import { deriveSyncEntry } from '../../frontend/src/home/state/sync-entry-view.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

const getAuthUserMock = vi.hoisted(() => vi.fn());
const startAuthLoginMock = vi.hoisted(() => vi.fn());
const signOutAuthMock = vi.hoisted(() => vi.fn());
const readVaultSessionMock = vi.hoisted(() => vi.fn());
const persistVaultSessionMock = vi.hoisted(() => vi.fn());

vi.mock('../../frontend/src/auth/oauth.ts', () => ({
  getAuthUser: (...args) => getAuthUserMock(...args),
  startAuthLogin: (...args) => startAuthLoginMock(...args),
  signOutAuth: (...args) => signOutAuthMock(...args),
  readVaultSession: (...args) => readVaultSessionMock(...args),
  persistVaultSession: (...args) => persistVaultSessionMock(...args),
}));

function readRel(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

function doneResult(overrides = {}) {
  return {
    ok: true,
    uploaded: 0,
    pulled: 0,
    skipped: 0,
    kept: 0,
    gone: 0,
    not_tried: 0,
    failed: [],
    stopped: null,
    cancelled: false,
    ...overrides,
  };
}

function signedUser(overrides = {}) {
  return {
    user_id: 'user-42',
    email: 'ada@example.com',
    display_name: 'Ada',
    avatar_url: 'https://example.com/a.png',
    provider: 'google',
    ...overrides,
  };
}

const entryOf = (overrides = {}) => ({
  id: 'a'.repeat(32),
  name: 'Notes',
  op: 'upload',
  state: 'not_started',
  reason: null,
  local: true,
  ...overrides,
});
const idOf = (n) => n.toString(16).padStart(32, '0');
const finished = (overrides) => ({ phase: 'ended', result: { kind: 'finished', summary: doneResult(overrides) } });
const none = { phase: 'none' };
const pending = { phase: 'pending', uploads: 2, pulls: 1 };

describe('sync button · source', () => {
  it('sits in the account row between the account bar and the ?', () => {
    const page = readRel('frontend/src/home/page.tsx');
    expect(page).toMatch(/from ['"]\.\/ui\/cloud-sync-button\.tsx['"]/);
    const rowStart = page.indexOf('data-role="account-row"');
    const account = page.indexOf('<AccountBar', rowStart);
    const sync = page.indexOf('<CloudSyncButton', rowStart);
    const help = page.indexOf('<HelpMenu', rowStart);
    expect(rowStart).toBeGreaterThan(-1);
    expect(sync).toBeGreaterThan(account);
    expect(help).toBeGreaterThan(sync);
    expect(page).toMatch(
      /<CloudSyncButton\s+entry=\{syncEntry\}\s+run=\{syncRun\}\s+roster=\{syncRoster\}\s+onStart=\{onSyncStart\}\s+onCancel=\{onSyncCancel\}\s+onViewOpen=\{markSyncResultSeen\}\s+onOpenDoc=\{\(id\) => void openDocById\(id\)\}/,
    );
  });

  it('is pure paint: the page composes, the button only shows and calls back', () => {
    const src = readRel('frontend/src/home/ui/cloud-sync-button.tsx');
    expect(src).toMatch(/export function CloudSyncButton/);
    expect(src).toMatch(/<HomeNavIcon name="sync"/);
    expect(src).not.toMatch(
      /syncDocuments|cancelSyncDocuments|checkSyncDocuments|getSyncProgress|refreshAuthSession|startSync|cancelSync|checkSyncPending|syncRunStore|\/host\//,
    );

    const page = readRel('frontend/src/home/page.tsx');
    expect(page).toMatch(/startSync/);
    expect(page).toMatch(/cancelSync/);
    expect(page).toMatch(/resetSyncEntry/);
    expect(page).toMatch(/checkSyncPending/);
    expect(page).toMatch(/deriveSyncEntry\(syncRun, syncCheck, syncSeen\)/);
  });

  it('has an icon of its own, so the button does not borrow another', () => {
    expect(readRel('frontend/src/home/ui/nav-icons.tsx')).toMatch(/^\s*sync: \(/m);
  });

  it('never turns the icon: the dot fades in and out, and only Start turns', () => {
    const css = readRel('frontend/app.css');
    expect(css).not.toMatch(/home-sync-turn|home-sync-breathe|data-turning/);
    // The pulse moves opacity and nothing else.
    const pulse = css.slice(css.indexOf('@keyframes home-sync-pulse'), css.indexOf('/* The menu of the icon. */'));
    expect(pulse).toMatch(/0%, 100%\s*\{\s*opacity:\s*0;\s*\}\s*50%\s*\{\s*opacity:\s*1;\s*\}/);
    expect(pulse).not.toMatch(/transform|rotate|scale|color|background/);
    expect(css).toMatch(
      /\.home-sync-button-trigger\[data-dot="pulse"\]::after\s*\{[^}]*animation:\s*home-sync-pulse[^}]*infinite/,
    );
    expect(css).toMatch(/\.home-sync-button-trigger\[data-dot="dot"\]::after\s*\{[^}]*opacity:\s*1/);
    expect(css).toMatch(/\.home-sync-button-trigger::after\s*\{[^}]*background:\s*var\(--fg-danger\)/);
    expect(css).toMatch(/\.home-sync-dot\s*\{[^}]*background:\s*var\(--fg-danger\)/);
    expect(css).toMatch(/\.home-sync-spinner\s*\{[^}]*animation:\s*home-sync-spin[^}]*infinite/);
    expect(css).toMatch(/@keyframes home-sync-spin\s*\{[^}]*rotate\(360deg\)/);
  });

  it('lights the dot steadily, and keeps the ring still, for those who ask for no motion', () => {
    const css = readRel('frontend/app.css');
    const block = css.slice(css.indexOf('@media (prefers-reduced-motion: reduce) {\n  .home-sync-button-trigger::after'));
    expect(block).toMatch(/\[data-dot="pulse"\]::after\s*\{[^}]*animation:\s*none[^}]*opacity:\s*1/);
    expect(block).toMatch(/\.home-sync-spinner\s*\{[^}]*animation:\s*none/);
  });

  it('opens its card over the whole account row, which holds it', () => {
    const css = readRel('frontend/app.css');
    expect(css).toMatch(/\.home-account-row\s*\{[^}]*position:\s*relative/);
    expect(css).toMatch(/\.home-sync-button-panel\s*\{[^}]*position:\s*absolute/);
    expect(css).toMatch(/\.home-sync-menu\s*\{[^}]*position:\s*absolute/);
  });

  it('is about 320px wide, and its list scrolls inside it so Cancel stays in view', () => {
    const css = readRel('frontend/app.css');
    expect(css).toMatch(/\.home-sync-button-panel\s*\{[^}]*min-width:\s*min\(320px,/);
    expect(css).toMatch(/\.home-sync-roster\s*\{[^}]*max-height:\s*\d+px[^}]*overflow-y:\s*auto/);
    expect(css).toMatch(/\.home-sync-group\[data-group="failed"\] \.home-sync-group-title\s*\{[^}]*color:\s*var\(--fg-danger\)/);
    expect(css).toMatch(/\.home-sync-row-name\[data-disabled="true"\]\s*\{[^}]*color:\s*var\(--fg-subtle\)/);
  });
});

describe('CloudSyncButton', () => {
  let container;
  let root;
  let CloudSyncButton;
  let onStart;
  let onCancel;
  let onViewOpen;
  let onOpenDoc;

  beforeEach(async () => {
    ({ CloudSyncButton } = await import('../../frontend/src/home/ui/cloud-sync-button.tsx'));
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    onStart = vi.fn();
    onCancel = vi.fn();
    onViewOpen = vi.fn();
    onOpenDoc = vi.fn();
  });

  afterEach(() => {
    act(() => {
      root?.unmount();
    });
    container?.remove();
  });

  /** Renders the button for a state of the three stores, the way the page derives it. */
  function render({ run = { phase: 'idle' }, check = none, seen = 'seen', entries = [] } = {}) {
    act(() => {
      root.render(
        createElement(CloudSyncButton, {
          entry: deriveSyncEntry(run, check, seen),
          run,
          roster: { entries },
          onStart,
          onCancel,
          onViewOpen,
          onOpenDoc,
        }),
      );
    });
  }

  const q = (role) => container.querySelector(`[data-role="${role}"]`);
  const trigger = () => q('sync-trigger');
  const title = () => q('sync-status-title')?.textContent;
  const detail = () => q('sync-status-detail')?.textContent;
  const cancel = () => q('sync-cancel');
  const click = (el) => {
    act(() => {
      el.click();
    });
  };
  const press = () => click(trigger());
  const openView = () => {
    press();
    click(q('sync-view'));
  };

  const running = { phase: 'running' };
  const cancelling = { phase: 'cancelling' };

  it('is an icon button that does nothing until it is pressed, and then only opens the menu', () => {
    render();

    expect(trigger().tagName).toBe('BUTTON');
    expect(trigger().getAttribute('aria-label')).toBe('Sync');
    expect(trigger().getAttribute('aria-haspopup')).toBe('menu');
    expect(trigger().getAttribute('aria-expanded')).toBe('false');
    expect(trigger().querySelector('[data-home-icon="sync"]')).not.toBeNull();
    expect(q('sync-menu')).toBeNull();

    press();

    expect(trigger().getAttribute('aria-expanded')).toBe('true');
    expect(q('sync-menu').getAttribute('role')).toBe('menu');
    expect(q('sync-start').textContent).toBe('Start');
    expect(q('sync-view').textContent).toBe('View');
    expect(onStart).not.toHaveBeenCalled();
    expect(onViewOpen).not.toHaveBeenCalled();
    expect(q('sync-status')).toBeNull();
  });

  it('closes the menu with a second press on the icon, and with a press outside it', () => {
    render();
    press();
    press();
    expect(q('sync-menu')).toBeNull();

    press();
    click(q('close-sync-menu'));

    expect(q('sync-menu')).toBeNull();
    expect(trigger().getAttribute('aria-expanded')).toBe('false');
    expect(onStart).not.toHaveBeenCalled();
  });

  // The seven states of the entry: [name, run, check, seen] -> [icon, start dot, start spins, start on, view dot, view on].
  const states = [
    ['1 · idle, nothing to sync', { phase: 'idle' }, none, 'seen', ['none', false, false, true, false, false]],
    ['2 · idle, something to sync', { phase: 'idle' }, pending, 'seen', ['dot', true, false, true, false, false]],
    ['3 · a round goes on', running, none, 'seen', ['pulse', false, true, false, false, true]],
    ['4 · ended, nothing behind, unseen', finished({ uploaded: 1 }), none, 'unseen', ['dot', false, false, true, true, true]],
    ['5 · ended, something behind, unseen', finished({ uploaded: 1 }), pending, 'unseen', ['dot', true, false, true, true, true]],
    ['6 · ended, something behind, seen', finished({ uploaded: 1 }), pending, 'seen', ['dot', true, false, true, false, true]],
    ['7 · ended, nothing behind, seen', finished({ uploaded: 1 }), none, 'seen', ['none', false, false, true, false, true]],
  ];

  it.each(states)('shows state %s', (_name, run, check, seen, expected) => {
    render({ run, check, seen });
    press();

    const shown = [
      trigger().getAttribute('data-dot'),
      q('sync-start-dot') !== null,
      q('sync-start-spinner') !== null,
      !q('sync-start').disabled,
      q('sync-view-dot') !== null,
      !q('sync-view').disabled,
    ];
    expect(shown).toEqual(expected);
  });

  it('shows the states of a round that could not start and of one cancelled before the host: both can be viewed', () => {
    render({
      run: { phase: 'ended', result: { kind: 'could_not_start', reason: 'Sign in first to sync.' } },
      seen: 'unseen',
    });
    press();
    expect(q('sync-view').disabled).toBe(false);
    expect(q('sync-view-dot')).not.toBeNull();
    expect(trigger().getAttribute('data-dot')).toBe('dot');

    render({ run: { phase: 'ended', result: { kind: 'cancelled_before_start' } }, seen: 'seen' });
    expect(q('sync-view').disabled).toBe(false);
  });

  it('asks for a round when Start is pressed, once per press, and closes the menu', () => {
    render({ check: pending });
    press();

    click(q('sync-start'));

    expect(onStart).toHaveBeenCalledTimes(1);
    expect(onViewOpen).not.toHaveBeenCalled();
    expect(q('sync-menu')).toBeNull();
  });

  it('cannot start a second round while one goes on', () => {
    render({ run: running });
    press();

    click(q('sync-start'));

    expect(q('sync-start').disabled).toBe(true);
    expect(onStart).not.toHaveBeenCalled();
    expect(trigger().getAttribute('aria-busy')).toBe('true');
  });

  it('does not turn the icon while a round goes on: the trigger carries a pulsing dot, and Start the spinner', () => {
    render({ run: running });

    expect(trigger().getAttribute('data-dot')).toBe('pulse');
    expect(trigger().hasAttribute('data-turning')).toBe(false);
    expect(trigger().querySelector('[data-role="sync-start-spinner"]')).toBeNull();
    press();
    expect(q('sync-start-spinner')).not.toBeNull();
    expect(q('sync-start-spinner').getAttribute('aria-hidden')).toBe('true');
  });

  it('opens the card from View, with what the round is doing, and says the result was opened', () => {
    render({ run: running });

    openView();

    expect(onViewOpen).toHaveBeenCalledTimes(1);
    expect(q('sync-menu')).toBeNull();
    expect(title()).toBe('Syncing…');
    expect(detail()).toMatch(/^Cancel stops before the next document/);
    expect(q('sync-status').getAttribute('role')).toBe('status');
    expect(q('sync-status').getAttribute('data-tone')).toBe('neutral');
  });

  it('has nothing to open while View is not enabled', () => {
    render();
    press();

    click(q('sync-view'));

    expect(onViewOpen).not.toHaveBeenCalled();
    expect(q('sync-status')).toBeNull();
    expect(q('sync-menu')).not.toBeNull();
  });

  it('cancels from the card, once per press', () => {
    render({ run: running });
    openView();

    expect(cancel().textContent).toBe('Cancel');
    expect(cancel().disabled).toBe(false);
    click(cancel());

    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onStart).not.toHaveBeenCalled();
  });

  it('says cancelling and offers no second cancel once the user has asked', () => {
    render({ run: running });
    openView();

    render({ run: cancelling });

    expect(title()).toBe('Cancelling…');
    expect(cancel().textContent).toBe('Cancelling…');
    expect(cancel().disabled).toBe(true);
    click(cancel());
    expect(onCancel).not.toHaveBeenCalled();
    expect(trigger().getAttribute('data-dot')).toBe('pulse');
  });

  it('offers Cancel while the sign-in is refreshed, and the card says so', () => {
    render({ run: { phase: 'starting' } });
    openView();

    expect(title()).toBe('Starting…');
    expect(cancel()).not.toBeNull();
  });

  const ended = [
    ['a round that was made', finished({ uploaded: 1, skipped: 2 }), 'neutral', 'Sync finished', 'Uploaded 1 · Pulled 0 · Unchanged 2'],
    [
      'a round with a failed document',
      finished({ failed: [{ id: 'd', op: 'upload', error: 'too big' }] }),
      'warn',
      'Synced, with problems',
      'Uploaded 0 · Pulled 0 · Unchanged 0 · 1 failed · first: too big',
    ],
    ['a cancelled round', finished({ uploaded: 1, not_tried: 2, cancelled: true }), 'neutral', 'Sync cancelled', 'Uploaded 1 · Pulled 0 · Unchanged 0 · 2 not tried, sync again to continue'],
    ['a cancel before the host was asked', { phase: 'ended', result: { kind: 'cancelled_before_start' } }, 'neutral', 'Sync cancelled', 'Nothing was sent or changed.'],
    ['a round that could not start', { phase: 'ended', result: { kind: 'could_not_start', reason: 'Sign in first to sync.' } }, 'warn', 'Cannot sync', 'Sign in first to sync.'],
  ];

  it.each(ended)('after %s: View tells what happened and offers no cancel', (_name, run, tone, heading, text) => {
    render({ run, seen: 'unseen' });

    openView();

    expect(title()).toBe(heading);
    expect(detail()).toBe(text);
    expect(q('sync-status').getAttribute('data-tone')).toBe(tone);
    expect(cancel()).toBeNull();
    expect(trigger().getAttribute('aria-busy')).toBe('false');
  });

  it('counts a round that ends while its card is open as seen', () => {
    render({ run: running });
    openView();
    expect(onViewOpen).toHaveBeenCalledTimes(1);

    render({ run: finished({ uploaded: 1 }), seen: 'unseen' });

    expect(title()).toBe('Sync finished');
    expect(onViewOpen).toHaveBeenCalledTimes(2);
  });

  it('does not say seen for a result that is only waiting behind a closed card', () => {
    render({ run: running });

    render({ run: finished({ uploaded: 1 }), seen: 'unseen' });

    expect(onViewOpen).not.toHaveBeenCalled();
    expect(q('sync-status')).toBeNull();
  });

  it('closes the card on a press outside it, and the next press on the icon opens the menu', () => {
    render({ run: finished({ uploaded: 1 }), seen: 'unseen' });
    openView();
    expect(title()).toBe('Sync finished');

    click(q('close-sync-panel'));
    expect(q('sync-status')).toBeNull();
    expect(trigger().getAttribute('aria-expanded')).toBe('false');

    press();
    expect(q('sync-menu')).not.toBeNull();
    expect(onStart).not.toHaveBeenCalled();
  });

  it('closes the card when there is nothing to view any more (the user signed out)', () => {
    render({ run: finished({ uploaded: 1 }), seen: 'unseen' });
    openView();
    expect(title()).toBe('Sync finished');

    render({ run: { phase: 'idle' } });
    expect(q('sync-status')).toBeNull();

    // A new result does not bring the old card back by itself.
    render({ run: finished({ uploaded: 2 }), seen: 'unseen' });
    expect(q('sync-status')).toBeNull();
  });

  describe('the documents of the round', () => {
    const rows = () => [...container.querySelectorAll('[data-role="sync-row"]')];
    const groups = () => [...container.querySelectorAll('[data-role="sync-group"]')];
    const groupTitles = () => groups().map((g) => g.querySelector('[data-role="sync-group-title"]').textContent);
    const namesIn = (group) => [...group.querySelectorAll('[data-role="sync-row-name"]')].map((n) => n.textContent);

    const mixed = [
      entryOf({ id: idOf(1), name: 'Roadmap', op: 'upload', state: 'done' }),
      entryOf({ id: idOf(2), name: 'Ideas', op: 'upload', state: 'running' }),
      entryOf({ id: idOf(3), name: 'Broken', op: 'upload', state: 'failed', reason: 'could not pack document' }),
      entryOf({ id: idOf(4), name: 'Plan', op: 'pull', state: 'not_started', local: true }),
      entryOf({ id: idOf(5), name: '0badf00d', op: 'pull', state: 'not_started', local: false }),
    ];

    it('groups them as Failed, In progress, Done, Not started, and leaves out a group with nothing in it', () => {
      render({ run: running, entries: mixed });
      openView();

      expect(groupTitles()).toEqual(['Failed · 1', 'In progress · 1', 'Done · 1', 'Not started · 2']);
      expect(namesIn(groups()[0])).toEqual(['Broken']);
      expect(namesIn(groups()[3])).toEqual(['Plan', '0badf00d']);

      render({ run: running, entries: [entryOf({ id: idOf(9), name: 'Only', state: 'not_started' })] });
      expect(groupTitles()).toEqual(['Not started · 1']);
    });

    it('marks each row with an arrow: up for what is sent, down for what is brought here', () => {
      render({ run: running, entries: mixed });
      openView();

      const arrows = Object.fromEntries(
        rows().map((row) => [
          row.querySelector('[data-role="sync-row-name"]').textContent,
          row.querySelector('.home-sync-row-arrow'),
        ]),
      );
      expect(arrows.Ideas.textContent).toBe('↑');
      expect(arrows.Ideas.getAttribute('aria-label')).toBe('Upload');
      expect(arrows.Plan.textContent).toBe('↓');
      expect(arrows.Plan.getAttribute('aria-label')).toBe('Download');
    });

    it('moves a document between groups as the host\'s list changes, replacing the list as a whole', () => {
      const queued = entryOf({ id: idOf(7), name: 'Draft', state: 'not_started' });
      render({ run: running, entries: [queued] });
      openView();
      expect(groupTitles()).toEqual(['Not started · 1']);

      render({ run: running, entries: [{ ...queued, state: 'running' }] });
      expect(groupTitles()).toEqual(['In progress · 1']);

      render({ run: running, entries: [{ ...queued, state: 'done' }] });
      expect(groupTitles()).toEqual(['Done · 1']);

      render({ run: running, entries: [{ ...queued, state: 'failed', reason: 'too big' }] });
      expect(groupTitles()).toEqual(['Failed · 1']);
    });

    it('puts what the pointer rests on in a hover: the whole name, the id, and for a failure the reason', () => {
      render({ run: running, entries: mixed });
      openView();

      const byName = Object.fromEntries(
        rows().map((row) => [row.querySelector('[data-role="sync-row-name"]').textContent, row.getAttribute('data-tip')]),
      );
      expect(byName.Broken).toBe(`Broken\n${idOf(3)}\ncould not pack document`);
      expect(byName.Ideas).toBe(`Ideas\n${idOf(2)}`);
      expect(byName.Roadmap).toBe(`Roadmap\n${idOf(1)}`);
    });

    it('opens a document this machine has when its name is pressed: the panel closes first, the round goes on', () => {
      render({ run: running, entries: mixed });
      openView();
      const plan = rows().find((row) => row.textContent.includes('Plan')).querySelector('button');

      click(plan);

      expect(onOpenDoc).toHaveBeenCalledTimes(1);
      expect(onOpenDoc).toHaveBeenCalledWith(idOf(4));
      expect(q('sync-panel')).toBeNull();
      expect(onCancel).not.toHaveBeenCalled();
      expect(onStart).not.toHaveBeenCalled();
      expect(trigger().getAttribute('data-dot')).toBe('pulse');
    });

    it('does not open a document this machine has not: its name is grey, and is no button', () => {
      render({ run: running, entries: mixed });
      openView();
      const cloudOnly = rows().find((row) => row.textContent.includes('0badf00d'));
      const name = cloudOnly.querySelector('[data-role="sync-row-name"]');

      expect(name.tagName).not.toBe('BUTTON');
      expect(name.getAttribute('data-disabled')).toBe('true');
      click(name);

      expect(onOpenDoc).not.toHaveBeenCalled();
      expect(q('sync-panel')).not.toBeNull();
    });

    it('folds a long group into "N more", and opens it with a press', () => {
      const many = Array.from({ length: 8 }, (_, n) => entryOf({ id: idOf(n + 1), name: `Doc ${n + 1}`, state: 'done' }));
      render({ run: finished({ uploaded: 8 }), seen: 'unseen', entries: many });
      openView();
      const doneGroup = () => groups()[0];

      expect(namesIn(doneGroup())).toEqual(['Doc 1', 'Doc 2', 'Doc 3', 'Doc 4', 'Doc 5']);
      expect(doneGroup().querySelector('[data-role="sync-more"]').textContent).toBe('3 more');
      expect(groupTitles()).toEqual(['Done · 8']);

      click(doneGroup().querySelector('[data-role="sync-more"]'));

      expect(namesIn(doneGroup()).length).toBe(8);
      expect(doneGroup().querySelector('[data-role="sync-more"]')).toBeNull();
    });

    it('does not fold a group of five or fewer', () => {
      const five = Array.from({ length: 5 }, (_, n) => entryOf({ id: idOf(n + 1), name: `Doc ${n + 1}`, state: 'done' }));
      render({ run: finished({ uploaded: 5 }), seen: 'unseen', entries: five });
      openView();

      expect(namesIn(groups()[0]).length).toBe(5);
      expect(q('sync-more')).toBeNull();
    });

    it('keeps Cancel below the list while the round goes on, and takes it away when the round ends', () => {
      render({ run: running, entries: mixed });
      openView();
      const panel = q('sync-panel');

      expect(panel.lastElementChild).toBe(cancel());
      expect(q('sync-roster').compareDocumentPosition(cancel()) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();

      render({ run: finished({ uploaded: 1 }), seen: 'unseen', entries: mixed });
      expect(cancel()).toBeNull();
      expect(q('sync-roster')).not.toBeNull();
    });

    it('shows no list for a round that could not start: one line of the reason, nothing else', () => {
      render({
        run: { phase: 'ended', result: { kind: 'could_not_start', reason: 'Sign in first to sync.' } },
        seen: 'unseen',
      });
      openView();

      expect(title()).toBe('Cannot sync');
      expect(detail()).toBe('Sign in first to sync.');
      expect(q('sync-roster')).toBeNull();
      expect(cancel()).toBeNull();
    });

    it('names the failed documents in the list, so the summary only counts them', () => {
      const failed = { id: idOf(3), op: 'upload', error: 'could not pack document' };
      render({
        run: finished({ uploaded: 1, failed: [failed] }),
        seen: 'unseen',
        entries: mixed,
      });
      openView();

      expect(title()).toBe('Synced, with problems');
      expect(detail()).toBe('Uploaded 1 · Pulled 0 · Unchanged 0 · 1 failed');

      // Without the list (the read failed) the summary keeps the first reason.
      render({ run: finished({ uploaded: 1, failed: [failed] }), seen: 'unseen', entries: [] });
      expect(detail()).toBe('Uploaded 1 · Pulled 0 · Unchanged 0 · 1 failed · first: could not pack document');
    });

    it('stays usable when opening a document throws', () => {
      onOpenDoc.mockImplementation(() => {
        throw new Error('cannot open');
      });
      render({ run: running, entries: mixed });
      openView();

      click(rows().find((row) => row.textContent.includes('Plan')).querySelector('button'));

      expect(container.querySelectorAll('[data-role="sync-button"]').length).toBe(1);
      expect(q('sync-panel')).toBeNull();
      press();
      expect(q('sync-menu')).not.toBeNull();
    });
  });

  it('stays usable when a callback throws', () => {
    onStart.mockImplementation(() => {
      throw new Error('start failed');
    });
    onCancel.mockImplementation(() => {
      throw new Error('cancel failed');
    });
    onViewOpen.mockImplementation(() => {
      throw new Error('view failed');
    });
    render();
    press();
    click(q('sync-start'));

    render({ run: running });
    openView();
    click(cancel());

    expect(container.querySelectorAll('[data-role="sync-button"]').length).toBe(1);
    expect(title()).toBe('Syncing…');
  });
});

describe('sync button · from the home rail', () => {
  let container;
  let cleanup;
  let realFetch;
  let syncDocuments;
  let cancelSyncDocuments;
  let checkSyncDocuments;
  let getSyncProgress;

  const nowSeconds = () => Math.floor(Date.now() / 1000);
  const vaultSession = (expiresIn) => ({
    access_token: 'vault-access',
    refresh_token: 'vault-refresh',
    expires_at: nowSeconds() + expiresIn,
    user: { id: 'user-42', email: 'ada@example.com' },
  });
  const json = (status, body) =>
    new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });
  const refreshed = () =>
    json(200, {
      access_token: 'new-access',
      refresh_token: 'new-refresh',
      token_type: 'bearer',
      expires_in: 3600,
      user: { id: 'user-42', email: 'ada@example.com', app_metadata: {}, user_metadata: {} },
    });

  beforeEach(async () => {
    container = document.createElement('div');
    document.body.appendChild(container);
    cleanup = null;
    realFetch = globalThis.fetch;
    getAuthUserMock.mockReset();
    startAuthLoginMock.mockReset();
    signOutAuthMock.mockReset();
    readVaultSessionMock.mockReset();
    persistVaultSessionMock.mockReset();
    getAuthUserMock.mockResolvedValue(null);
    readVaultSessionMock.mockResolvedValue(null);
    persistVaultSessionMock.mockResolvedValue(true);
    signOutAuthMock.mockImplementation(async () => {
      const { authUserStore } = await import('../../frontend/src/auth/state/user.ts');
      authUserStore.set(null);
    });
    const { authUserStore } = await import('../../frontend/src/auth/state/user.ts');
    authUserStore.set(null);
    window.__TAURI__ = { event: { listen: vi.fn(async () => vi.fn()) } };
    vi.spyOn(api, 'getMessageChannelUnread').mockResolvedValue(false);
    vi.spyOn(api, 'markMessageChannelRead').mockResolvedValue(undefined);
    vi.spyOn(api, 'invoke').mockImplementation(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'unbound' };
      if (cmd === 'list_chat_sessions') return { sessions: [], current_session_id: '' };
      return {};
    });
    syncDocuments = vi
      .spyOn(api, 'syncDocuments')
      .mockResolvedValue(doneResult({ uploaded: 1, skipped: 2 }));
    cancelSyncDocuments = vi
      .spyOn(api, 'cancelSyncDocuments')
      .mockResolvedValue({ ok: true, cancelling: true });
    checkSyncDocuments = vi
      .spyOn(api, 'checkSyncDocuments')
      .mockResolvedValue({ ok: true, uploads: 0, pulls: 0 });
    getSyncProgress = vi
      .spyOn(api, 'getSyncProgress')
      .mockResolvedValue({ ok: true, running: false, entries: [] });
    globalThis.fetch = vi.fn(async () => {
      throw new Error('the network was not expected to be used');
    });
  });

  afterEach(async () => {
    cleanup?.();
    vi.restoreAllMocks();
    globalThis.fetch = realFetch;
    const { syncRunStore, syncCheckStore, syncSeenStore, syncRosterStore, EMPTY_ROSTER } = await import(
      '../../frontend/src/home/state/sync-entry.ts'
    );
    const { dropPendingSyncCheck } = await import('../../frontend/src/home/commands/sync-check.ts');
    dropPendingSyncCheck();
    syncRunStore.set({ phase: 'idle' });
    syncCheckStore.set({ phase: 'unchecked' });
    syncSeenStore.set('seen');
    syncRosterStore.set(EMPTY_ROSTER);
    container.remove();
    delete window.__TAURI__;
  });

  async function mountHome({ signedIn }) {
    if (signedIn) getAuthUserMock.mockResolvedValue(signedUser());
    const { mountHomeHub } = await import('../../frontend/src/home/hub.tsx');
    cleanup = mountHomeHub(container, { navigate: vi.fn() });
    await vi.waitFor(() => {
      expect(container.querySelector('[data-role="account-bar"]')).not.toBeNull();
    });
    if (signedIn) {
      await vi.waitFor(() => {
        expect(container.querySelector('[data-role="account-bar-name"]')?.textContent).toBe('Ada');
      });
    }
  }

  const q = (role) => container.querySelector(`[data-role="${role}"]`);
  const trigger = () => q('sync-trigger');
  const dot = () => trigger().getAttribute('data-dot');
  const title = () => q('sync-status-title')?.textContent;
  const detail = () => q('sync-status-detail')?.textContent;
  const click = (el) => {
    act(() => {
      el.click();
    });
  };
  /** Opens the menu if it is closed, then presses one of its items. */
  const choose = (role) => {
    if (!q('sync-menu')) click(trigger());
    click(q(role));
  };
  const openMenu = () => {
    if (!q('sync-menu')) click(trigger());
  };
  const later = (ms) => act(async () => { await new Promise((resolve) => setTimeout(resolve, ms)); });

  it('puts the icon beside the ?, and leaves the account menu to signing in and out', async () => {
    await mountHome({ signedIn: true });

    const row = q('account-row');
    const order = [...row.querySelectorAll('[data-role="account-bar"],[data-role="sync-button"],[data-role="help-menu"]')]
      .map((el) => el.getAttribute('data-role'));
    expect(order).toEqual(['account-bar', 'sync-button', 'help-menu']);
    click(q('account-bar-trigger'));
    expect(container.querySelector('[data-role="account-logout"]')).not.toBeNull();
    expect(container.querySelector('[data-role="account-sync"]')).toBeNull();
  });

  it('signed in, nothing behind: checks once, shows no dot, and does not sync until Start is pressed', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(3600));
    await mountHome({ signedIn: true });
    await vi.waitFor(() => {
      expect(checkSyncDocuments).toHaveBeenCalledTimes(1);
    });
    await later(20);

    expect(syncDocuments).not.toHaveBeenCalled();
    expect(dot()).toBe('none');
    expect(q('sync-status')).toBeNull();
    openMenu();
    expect(q('sync-start-dot')).toBeNull();
    expect(q('sync-start').disabled).toBe(false);
    expect(q('sync-view').disabled).toBe(true);
  });

  it('signed out: nothing is asked, and Start tells the user to sign in first', async () => {
    await mountHome({ signedIn: false });
    await later(20);
    expect(checkSyncDocuments).not.toHaveBeenCalled();
    expect(dot()).toBe('none');

    choose('sync-start');
    await vi.waitFor(() => {
      expect(dot()).toBe('dot');
    });

    choose('sync-view');
    expect(title()).toBe('Cannot sync');
    expect(detail()).toBe('Sign in first to sync.');
    expect(syncDocuments).not.toHaveBeenCalled();
  });

  it('something is behind: the icon and Start carry a dot before anything is pressed', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(3600));
    checkSyncDocuments.mockResolvedValue({ ok: true, uploads: 2, pulls: 1 });
    await mountHome({ signedIn: true });

    await vi.waitFor(() => {
      expect(dot()).toBe('dot');
    });
    openMenu();
    expect(q('sync-start-dot')).not.toBeNull();
    expect(q('sync-view-dot')).toBeNull();
    expect(q('sync-view').disabled).toBe(true);
    expect(syncDocuments).not.toHaveBeenCalled();
  });

  it('a round: the dot pulses and Start turns while the host works, then the result waits unseen behind a dot', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(3600));
    let finish;
    syncDocuments.mockReturnValue(new Promise((resolve) => { finish = resolve; }));
    await mountHome({ signedIn: true });

    choose('sync-start');

    await vi.waitFor(() => {
      expect(dot()).toBe('pulse');
    });
    for (let i = 0; i < 3; i += 1) {
      await later(30);
      expect(dot()).toBe('pulse');
      expect(trigger().getAttribute('aria-busy')).toBe('true');
    }
    openMenu();
    expect(q('sync-start-spinner')).not.toBeNull();
    expect(q('sync-start').disabled).toBe(true);
    expect(q('sync-view').disabled).toBe(false);
    click(q('sync-start'));
    expect(syncDocuments).toHaveBeenCalledTimes(1);
    click(trigger());

    await act(async () => {
      finish(doneResult({ uploaded: 1, skipped: 2 }));
    });

    await vi.waitFor(() => {
      expect(dot()).toBe('dot');
    });
    expect(trigger().getAttribute('aria-busy')).toBe('false');
    expect(globalThis.fetch).not.toHaveBeenCalled();
    openMenu();
    expect(q('sync-view-dot')).not.toBeNull();
    expect(q('sync-start-spinner')).toBeNull();

    click(q('sync-view'));

    expect(title()).toBe('Sync finished');
    expect(detail()).toBe('Uploaded 1 · Pulled 0 · Unchanged 2');
    expect(q('sync-cancel')).toBeNull();
    click(q('close-sync-panel'));
    // Seen, and nothing is behind any more: the entry is at rest.
    expect(dot()).toBe('none');
    openMenu();
    expect(q('sync-view-dot')).toBeNull();
    expect(q('sync-view').disabled).toBe(false);
  });

  it('reads the host\'s record while the round goes on and once more after it', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(3600));
    let finish;
    syncDocuments.mockReturnValue(new Promise((resolve) => { finish = resolve; }));
    await mountHome({ signedIn: true });

    choose('sync-start');
    await vi.waitFor(() => {
      expect(syncDocuments).toHaveBeenCalledTimes(1);
    });
    await vi.waitFor(() => {
      expect(getSyncProgress.mock.calls.length).toBeGreaterThanOrEqual(1);
    });
    const during = getSyncProgress.mock.calls.length;

    await act(async () => {
      finish(doneResult({ uploaded: 1 }));
    });
    await vi.waitFor(() => {
      expect(dot()).toBe('dot');
    });

    expect(getSyncProgress.mock.calls.length).toBeGreaterThan(during);
  });

  it('shows the round that goes on from View, with Cancel, and when it ends while open it counts as seen', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(3600));
    let finish;
    syncDocuments.mockReturnValue(new Promise((resolve) => { finish = resolve; }));
    await mountHome({ signedIn: true });
    choose('sync-start');
    await vi.waitFor(() => {
      expect(syncDocuments).toHaveBeenCalledTimes(1);
    });

    choose('sync-view');
    expect(title()).toBe('Syncing…');
    expect(q('sync-cancel')).not.toBeNull();

    await act(async () => {
      finish(doneResult({ uploaded: 1, skipped: 2 }));
    });

    await vi.waitFor(() => {
      expect(title()).toBe('Sync finished');
    });
    expect(q('sync-cancel')).toBeNull();
    // Opened while it ended: no dot is left for a result the user is reading.
    await vi.waitFor(() => {
      expect(dot()).toBe('none');
    });
  });

  it('cancel: the host is asked, the card says cancelling, and the answer says what was done', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(3600));
    let finish;
    syncDocuments.mockReturnValue(new Promise((resolve) => { finish = resolve; }));
    await mountHome({ signedIn: true });
    choose('sync-start');
    await vi.waitFor(() => {
      expect(syncDocuments).toHaveBeenCalledTimes(1);
    });
    choose('sync-view');
    expect(q('sync-cancel')).not.toBeNull();

    click(q('sync-cancel'));

    await vi.waitFor(() => {
      expect(title()).toBe('Cancelling…');
    });
    expect(cancelSyncDocuments).toHaveBeenCalledTimes(1);
    expect(q('sync-cancel').disabled).toBe(true);
    expect(dot()).toBe('pulse');

    await act(async () => {
      finish(doneResult({ uploaded: 1, not_tried: 3, cancelled: true }));
    });

    await vi.waitFor(() => {
      expect(title()).toBe('Sync cancelled');
    });
    expect(detail()).toBe('Uploaded 1 · Pulled 0 · Unchanged 0 · 3 not tried, sync again to continue');
    expect(q('sync-cancel')).toBeNull();
    expect(trigger().getAttribute('aria-busy')).toBe('false');
  });

  it('cancel while the sign-in is being refreshed: the refresh is kept, the host is never asked to sync', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(30));
    let finishFetch;
    globalThis.fetch = vi.fn(() => new Promise((resolve) => { finishFetch = () => resolve(refreshed()); }));
    await mountHome({ signedIn: true });
    // The start-up check waits for the same refresh; let it be answered before the round begins.
    await vi.waitFor(() => {
      expect(globalThis.fetch).toHaveBeenCalled();
    });
    await act(async () => {
      finishFetch();
    });
    await vi.waitFor(() => {
      expect(checkSyncDocuments).toHaveBeenCalledTimes(1);
    });
    readVaultSessionMock.mockResolvedValue(vaultSession(30));
    globalThis.fetch = vi.fn(() => new Promise((resolve) => { finishFetch = () => resolve(refreshed()); }));

    choose('sync-start');
    await vi.waitFor(() => {
      expect(globalThis.fetch).toHaveBeenCalled();
    });
    choose('sync-view');
    expect(title()).toBe('Starting…');

    click(q('sync-cancel'));
    await vi.waitFor(() => {
      expect(title()).toBe('Sync cancelled');
    });
    expect(detail()).toBe('Nothing was sent or changed.');
    await act(async () => {
      finishFetch();
    });
    await later(20);

    expect(syncDocuments).not.toHaveBeenCalled();
    expect(cancelSyncDocuments).not.toHaveBeenCalled();
  });

  it('a session about to expire is refreshed and stored first, then the host syncs', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(30));
    globalThis.fetch = vi.fn(async () => refreshed());
    await mountHome({ signedIn: true });
    await vi.waitFor(() => {
      expect(checkSyncDocuments).toHaveBeenCalledTimes(1);
    });
    persistVaultSessionMock.mockClear();
    syncDocuments.mockClear();

    choose('sync-start');
    await vi.waitFor(() => {
      expect(syncDocuments).toHaveBeenCalledTimes(1);
    });

    expect(persistVaultSessionMock).toHaveBeenCalled();
    expect(persistVaultSessionMock.mock.calls[0][0]).toMatchObject({
      access_token: 'new-access',
      refresh_token: 'new-refresh',
    });
    expect(persistVaultSessionMock.mock.invocationCallOrder[0]).toBeLessThan(
      syncDocuments.mock.invocationCallOrder[0],
    );
  });

  it('a session that cannot be refreshed: View says to sign in again, and the host is not asked', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(30));
    globalThis.fetch = vi.fn(async () =>
      json(400, { error: 'invalid_grant', error_description: 'Invalid Refresh Token: Already Used' }),
    );
    await mountHome({ signedIn: true });
    await vi.waitFor(() => {
      expect(globalThis.fetch).toHaveBeenCalled();
    });

    choose('sync-start');

    await vi.waitFor(() => {
      expect(dot()).toBe('dot');
    });
    choose('sync-view');
    expect(title()).toBe('Cannot sync');
    expect(detail()).toBe('Your sign-in is no longer valid. Sign in again to sync.');
    expect(syncDocuments).not.toHaveBeenCalled();
    expect(persistVaultSessionMock).not.toHaveBeenCalled();
    expect(trigger().getAttribute('aria-busy')).toBe('false');
  });

  it('shows the reason when the host cannot make a sync, with a dot until it is read', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(3600));
    const refused = new Error('Cannot sync: cloud sync not reachable: connection refused');
    refused.status = 502;
    syncDocuments.mockRejectedValue(refused);
    await mountHome({ signedIn: true });

    choose('sync-start');

    await vi.waitFor(() => {
      expect(dot()).toBe('dot');
    });
    openMenu();
    expect(q('sync-view-dot')).not.toBeNull();
    click(q('sync-view'));
    expect(title()).toBe('Cannot sync');
    expect(detail()).toBe('Cannot sync: cloud sync not reachable: connection refused');
    expect(q('sync-status').getAttribute('data-tone')).toBe('warn');
  });

  it('can start again after a round has ended, and the check is asked again at the end of each', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(3600));
    await mountHome({ signedIn: true });
    await vi.waitFor(() => {
      expect(checkSyncDocuments).toHaveBeenCalledTimes(1);
    });

    choose('sync-start');
    await vi.waitFor(() => {
      expect(checkSyncDocuments).toHaveBeenCalledTimes(2);
    });
    choose('sync-start');
    await vi.waitFor(() => {
      expect(syncDocuments).toHaveBeenCalledTimes(2);
    });
    await vi.waitFor(() => {
      expect(checkSyncDocuments).toHaveBeenCalledTimes(3);
    });
  });

  it('what is still behind after a round keeps a dot on Start, though the result was read', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(3600));
    syncDocuments.mockResolvedValue(doneResult({ failed: [{ id: 'd', op: 'upload', error: 'too big' }] }));
    await mountHome({ signedIn: true });
    await vi.waitFor(() => {
      expect(checkSyncDocuments).toHaveBeenCalledTimes(1);
    });
    checkSyncDocuments.mockResolvedValue({ ok: true, uploads: 1, pulls: 0 });

    choose('sync-start');
    await vi.waitFor(() => {
      expect(dot()).toBe('dot');
    });
    choose('sync-view');
    expect(title()).toBe('Synced, with problems');
    expect(q('sync-status').getAttribute('data-tone')).toBe('warn');
    click(q('close-sync-panel'));

    // Read, but one document is still behind: states 5 and 6 of the table.
    expect(dot()).toBe('dot');
    openMenu();
    expect(q('sync-start-dot')).not.toBeNull();
    expect(q('sync-view-dot')).toBeNull();
  });

  it('lists the documents as the host\'s record moves them along, and a name this machine has opens its document', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(3600));
    let finish;
    syncDocuments.mockReturnValue(new Promise((resolve) => { finish = resolve; }));
    const resolveDocForOpen = vi.spyOn(api, 'resolveDocForOpen').mockRejectedValue(new Error('not here'));
    const ideas = { id: 'b'.repeat(32), name: 'Ideas', op: 'upload', state: 'not_started', reason: null, local: true };
    const cloudOnly = { id: 'c'.repeat(32), name: 'c'.repeat(8), op: 'pull', state: 'not_started', reason: null, local: false };
    getSyncProgress.mockResolvedValue({ ok: true, running: true, entries: [ideas, cloudOnly] });
    await mountHome({ signedIn: true });
    choose('sync-start');
    await vi.waitFor(() => {
      expect(syncDocuments).toHaveBeenCalledTimes(1);
    });
    choose('sync-view');
    const titles = () => [...container.querySelectorAll('[data-role="sync-group-title"]')].map((n) => n.textContent);

    // The panel is read once per poll: what the host lists is what the panel shows.
    await vi.waitFor(() => {
      expect(titles()).toEqual(['Not started · 2']);
    }, { timeout: 3000 });
    getSyncProgress.mockResolvedValue({ ok: true, running: true, entries: [{ ...ideas, state: 'running' }, cloudOnly] });
    await vi.waitFor(() => {
      expect(titles()).toEqual(['In progress · 1', 'Not started · 1']);
    }, { timeout: 3000 });

    // A name this machine has opens the document and takes the panel away; the round is not touched.
    click(container.querySelector('[data-group="running"] button[data-role="sync-row-name"]'));
    await vi.waitFor(() => {
      expect(resolveDocForOpen).toHaveBeenCalledWith(ideas.id);
    });
    expect(q('sync-panel')).toBeNull();
    expect(dot()).toBe('pulse');
    expect(cancelSyncDocuments).not.toHaveBeenCalled();

    // The round ends: the last read of the record is what the panel shows.
    getSyncProgress.mockResolvedValue({
      ok: true,
      running: false,
      entries: [{ ...ideas, state: 'done' }, { ...cloudOnly, state: 'failed', reason: 'cloud sync not reachable', local: false }],
    });
    await act(async () => {
      finish(doneResult({ uploaded: 1, failed: [{ id: cloudOnly.id, op: 'pull', error: 'cloud sync not reachable' }] }));
    });
    await vi.waitFor(() => {
      expect(dot()).toBe('dot');
    });
    choose('sync-view');
    expect(titles()).toEqual(['Failed · 1', 'Done · 1']);
    expect(detail()).toBe('Uploaded 1 · Pulled 0 · Unchanged 0 · 1 failed');
    const failedRow = container.querySelector('[data-group="failed"] [data-role="sync-row"]');
    expect(failedRow.getAttribute('data-tip')).toBe(`${'c'.repeat(8)}\n${cloudOnly.id}\ncloud sync not reachable`);
    expect(failedRow.querySelector('button')).toBeNull();
  });

  it('forgets the last round when the user signs out', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(3600));
    syncDocuments.mockResolvedValue(doneResult({ failed: [{ id: 'd', op: 'upload', error: 'too big' }] }));
    await mountHome({ signedIn: true });
    choose('sync-start');
    await vi.waitFor(() => {
      expect(dot()).toBe('dot');
    });
    choose('sync-view');
    expect(title()).toBe('Synced, with problems');
    click(q('close-sync-panel'));

    click(q('account-bar-trigger'));
    click(container.querySelector('[data-role="account-logout"]'));
    await vi.waitFor(() => {
      expect(signOutAuthMock).toHaveBeenCalledTimes(1);
      expect(q('account-bar-name')).toBeNull();
    });

    await vi.waitFor(() => {
      expect(dot()).toBe('none');
    });
    expect(q('sync-status')).toBeNull();
    openMenu();
    expect(q('sync-view').disabled).toBe(true);
    expect(q('sync-view-dot')).toBeNull();
  });
});
