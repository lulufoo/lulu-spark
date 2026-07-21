// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

import { createContentRegistry } from '../frontend/js/home-entry-shell/content-registry.js';
import { mountHomeEntryShell } from '../frontend/js/home-entry-shell/shell.js';

/**
 * Failure / recovery paths for the home-entry shell (T4).
 * Content load failure stays on B; exception / close failure force A;
 * shell exposes A/B/C snapshots; OverlayChrome title from EntryConfig only.
 */

const ENTRY_OK = {
  id: 'ok-entry',
  contentKey: 'ok-content',
  title: 'OK Entry',
  overlayTitle: 'OK Overlay',
};

const ENTRY_MISSING = {
  id: 'missing-entry',
  contentKey: 'missing-key',
  title: 'Missing Entry',
  overlayTitle: 'Missing Overlay',
};

const ENTRY_THROW = {
  id: 'throw-entry',
  contentKey: 'throw-content',
  title: 'Throw Entry',
  overlayTitle: 'Throw Overlay',
};

function okAdapter() {
  return {
    mount(slot, ctx) {
      const marker = document.createElement('div');
      marker.className = 'shell-content-marker';
      marker.dataset.entryId = ctx.entry.id;
      marker.textContent = `content:${ctx.entry.id}`;
      slot.appendChild(marker);
      return {
        unmount() {
          marker.remove();
        },
      };
    },
  };
}

function throwingAdapter(error = new Error('content load failed')) {
  return {
    mount() {
      throw error;
    },
  };
}

/**
 * @param {{
 *   entries?: typeof ENTRY_OK[],
 *   registerOk?: boolean,
 *   registerThrow?: boolean,
 *   closeUnmountThrows?: boolean,
 * }} [opts]
 */
function mountFixture(opts = {}) {
  const {
    entries = [ENTRY_OK, ENTRY_MISSING, ENTRY_THROW],
    registerOk = true,
    registerThrow = true,
    closeUnmountThrows = false,
  } = opts;

  const registry = createContentRegistry();
  if (registerOk) {
    registry.register(ENTRY_OK.contentKey, okAdapter());
  }
  if (registerThrow) {
    if (closeUnmountThrows) {
      registry.register(ENTRY_THROW.contentKey, {
        mount(slot, ctx) {
          const marker = document.createElement('div');
          marker.className = 'shell-content-marker';
          marker.dataset.entryId = ctx.entry.id;
          slot.appendChild(marker);
          return {
            unmount() {
              throw new Error('close failed');
            },
          };
        },
      });
    } else {
      registry.register(ENTRY_THROW.contentKey, throwingAdapter());
    }
  }

  const anchor = document.createElement('div');
  document.body.appendChild(anchor);
  const shell = mountHomeEntryShell(anchor, {
    config: entries,
    registry,
    host: {},
  });
  return { anchor, shell, registry };
}

function hub(root) {
  return root.querySelector('[data-role="hub"]');
}

function entryBtn(root, entryId) {
  return root.querySelector(`[data-entry-id="${entryId}"]`);
}

function overlay(root) {
  return root.querySelector('[data-role="overlay"]');
}

function titleEl(root) {
  return root.querySelector('[data-role="title"]');
}

function contentSlot(root) {
  return root.querySelector('[data-role="content-slot"]');
}

function contentError(root) {
  return root.querySelector('[data-role="content-error"]');
}

function closeBtn(root) {
  return root.querySelector('[data-role="close"]');
}

async function openHubThenEntry(anchor, shell, entryId) {
  if (shell.getState().mode === 'A') {
    hub(anchor).click();
  }
  expect(shell.getState().mode).toBe('B');
  if (typeof shell.openContent === 'function') {
    await shell.openContent(entryId);
    return;
  }
  entryBtn(anchor, entryId).click();
  await Promise.resolve();
  await Promise.resolve();
}

describe('home-entry-shell recovery · content failure stays B; force A; snapshot (T4)', () => {
  /** @type {{ anchor: HTMLElement, shell: ReturnType<typeof mountHomeEntryShell> } | null} */
  let fx = null;

  beforeEach(() => {
    document.body.innerHTML = '';
  });

  afterEach(() => {
    fx?.shell.unmount();
    fx = null;
    document.body.innerHTML = '';
  });

  it('registered + loadable content reaches success C; slot renders business; chrome title from EntryConfig', async () => {
    fx = mountFixture();
    const { anchor, shell } = fx;

    await openHubThenEntry(anchor, shell, ENTRY_OK.id);

    expect(shell.getState()).toEqual({ mode: 'C', entryId: ENTRY_OK.id });
    expect(overlay(anchor).hidden).toBe(false);
    expect(titleEl(anchor).textContent).toBe(ENTRY_OK.overlayTitle);
    const marker = contentSlot(anchor).querySelector('.shell-content-marker');
    expect(marker).not.toBeNull();
    expect(marker.dataset.entryId).toBe(ENTRY_OK.id);
    expect(contentError(anchor)).toBeNull();
  });

  it('shell exposes state snapshots for A / B / C (entryId only when C)', async () => {
    fx = mountFixture();
    const { anchor, shell } = fx;

    expect(shell.getState()).toEqual({ mode: 'A' });

    hub(anchor).click();
    expect(shell.getState()).toEqual({ mode: 'B' });

    await openHubThenEntry(anchor, shell, ENTRY_OK.id);
    expect(shell.getState()).toEqual({ mode: 'C', entryId: ENTRY_OK.id });
  });

  it('missing contentKey: stays on B with title + error placeholder; does not force A; does not occupy success C', async () => {
    fx = mountFixture();
    const { anchor, shell } = fx;

    await openHubThenEntry(anchor, shell, ENTRY_MISSING.id);

    expect(shell.getState()).toEqual({ mode: 'B' });
    expect(shell.getState().mode).not.toBe('A');
    expect(shell.getState().mode).not.toBe('C');
    expect(titleEl(anchor).textContent).toBe(ENTRY_MISSING.overlayTitle);
    expect(contentError(anchor)).not.toBeNull();
    expect(contentSlot(anchor).querySelector('.shell-content-marker')).toBeNull();
  });

  it('content load failure: stays on B with error placeholder; never success C; never forced A', async () => {
    fx = mountFixture();
    const { anchor, shell } = fx;

    await openHubThenEntry(anchor, shell, ENTRY_THROW.id);

    expect(shell.getState()).toEqual({ mode: 'B' });
    expect(titleEl(anchor).textContent).toBe(ENTRY_THROW.overlayTitle);
    expect(contentError(anchor)).not.toBeNull();
    expect(contentSlot(anchor).querySelector('.shell-content-marker')).toBeNull();
  });

  it('forceRecoverA (exception path) collapses to A and clears overlay/content', async () => {
    fx = mountFixture();
    const { anchor, shell } = fx;

    await openHubThenEntry(anchor, shell, ENTRY_OK.id);
    expect(shell.getState().mode).toBe('C');

    expect(typeof shell.forceRecoverA).toBe('function');
    shell.forceRecoverA('exception');

    expect(shell.getState()).toEqual({ mode: 'A' });
    expect(overlay(anchor).hidden).toBe(true);
    expect(contentSlot(anchor).children).toHaveLength(0);
    expect(titleEl(anchor).textContent).toBe('');
  });

  it('close failure forces A (does not stay on success C)', async () => {
    fx = mountFixture({
      entries: [ENTRY_THROW],
      registerOk: false,
      registerThrow: true,
      closeUnmountThrows: true,
    });
    const { anchor, shell } = fx;

    await openHubThenEntry(anchor, shell, ENTRY_THROW.id);
    expect(shell.getState()).toEqual({ mode: 'C', entryId: ENTRY_THROW.id });

    closeBtn(anchor).click();
    await Promise.resolve();
    await Promise.resolve();

    expect(shell.getState()).toEqual({ mode: 'A' });
    expect(overlay(anchor).hidden).toBe(true);
  });

  it('failure paths never impersonate success C (no business marker + mode C)', async () => {
    fx = mountFixture();
    const { anchor, shell } = fx;

    await openHubThenEntry(anchor, shell, ENTRY_MISSING.id);
    const missingState = shell.getState();
    expect(missingState.mode).not.toBe('C');
    expect(contentSlot(anchor).querySelector('.shell-content-marker')).toBeNull();

    await openHubThenEntry(anchor, shell, ENTRY_THROW.id);
    const throwState = shell.getState();
    expect(throwState.mode).not.toBe('C');
    expect(contentSlot(anchor).querySelector('.shell-content-marker')).toBeNull();
  });

  // T7: leave-host / unmount recovery forces A (paired with main-wire leave path)
  it('unmount while in C forces A (leave-host recovery; overlay must not survive)', async () => {
    fx = mountFixture();
    const { anchor, shell } = fx;

    await openHubThenEntry(anchor, shell, ENTRY_OK.id);
    expect(shell.getState().mode).toBe('C');

    shell.unmount();
    expect(shell.getState()).toEqual({ mode: 'A' });
    fx = null;
  });

  it('OverlayChrome title is wired from EntryConfig display fields only (not business internals)', async () => {
    const custom = {
      id: 'custom',
      contentKey: 'ok-content',
      title: 'Custom Title',
      overlayTitle: 'Custom Overlay Title',
    };
    const spyMount = vi.fn((slot, ctx) => {
      const el = document.createElement('div');
      el.className = 'shell-content-marker';
      el.dataset.secret = 'business-internal';
      slot.appendChild(el);
      return { unmount() { el.remove(); } };
    });
    const registry = createContentRegistry();
    registry.register('ok-content', { mount: spyMount });
    const anchor = document.createElement('div');
    document.body.appendChild(anchor);
    const shell = mountHomeEntryShell(anchor, {
      config: [custom],
      registry,
      host: {},
    });
    fx = { anchor, shell };

    await openHubThenEntry(anchor, shell, custom.id);

    expect(titleEl(anchor).textContent).toBe('Custom Overlay Title');
    expect(titleEl(anchor).textContent).not.toContain('business-internal');
    expect(spyMount).toHaveBeenCalled();
  });
});
