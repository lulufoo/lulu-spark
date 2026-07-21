// @vitest-environment jsdom
/**
 * T5: main.js once-mount wiring + host callbacks + leave/unmount force A;
 * remove old four-FAB mutex / CSS stack.
 */
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { createContentRegistry } from '../frontend/js/home-entry-shell/content-registry.js';
import { mountHomeEntryShell } from '../frontend/js/home-entry-shell/shell.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');

function readMain() {
  return readFileSync(join(repoRoot, 'frontend/js/main.js'), 'utf8');
}

function readAppCss() {
  return readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');
}

/** Count top-level mount*AssistantWidget(document.body …) calls in main.js. */
function countLegacyBodyMounts(source) {
  const re =
    /mount(?:ReadLater|PlanTask|Note|Builders)AssistantWidget\s*\(\s*document\.body\b/g;
  return (source.match(re) || []).length;
}

/** Detect the retired document capture-phase FAB mutex listener. */
function hasFabMutexListener(source) {
  return (
    /closest\(\s*['"]\.rl-assistant-fab['"]\s*\)/.test(source) &&
    /closest\(\s*['"]\.pt-assistant-fab['"]\s*\)/.test(source) &&
    /closest\(\s*['"]\.note-assistant-fab['"]\s*\)/.test(source) &&
    /closest\(\s*['"]\.builders-entry-fab['"]\s*\)/.test(source)
  );
}

function mountFixture() {
  const entry = {
    id: 'wire-entry',
    contentKey: 'wire-content',
    title: 'Wire',
    overlayTitle: 'Wire Overlay',
  };
  const registry = createContentRegistry();
  registry.register(entry.contentKey, {
    mount(slot, ctx) {
      const marker = document.createElement('div');
      marker.className = 'shell-content-marker';
      marker.dataset.entryId = ctx.entry.id;
      slot.appendChild(marker);
      return {
        unmount() {
          marker.remove();
        },
      };
    },
  });
  const anchor = document.createElement('div');
  document.body.appendChild(anchor);
  const shell = mountHomeEntryShell(anchor, {
    config: [entry],
    registry,
    host: {},
  });
  return { anchor, shell, entry };
}

describe('home-entry-shell main wiring (T5)', () => {
  it('main.js mounts home entry shell once on document.body with config/registry/host', () => {
    const source = readMain();
    const mounts = source.match(/mountHomeEntryShell\s*\(\s*document\.body\b/g) || [];
    expect(mounts.length, 'exactly one document.body shell mount').toBe(1);
    expect(source).toMatch(/mountHomeEntryShell\s*\(\s*document\.body\s*,\s*\{/);
    expect(source).toMatch(/\bconfig\b/);
    expect(source).toMatch(/\bregistry\b/);
    expect(source).toMatch(/\bhost\b/);
  });

  it('main.js removes the four mount*AssistantWidget(document.body) orchestration points', () => {
    expect(countLegacyBodyMounts(readMain())).toBe(0);
  });

  it('main.js removes the document-level FAB mutual-exclusion capture listener', () => {
    expect(hasFabMutexListener(readMain())).toBe(false);
  });

  it('host retains navigate, openReadLater (→ openReadLaterDialog), and openCreateNote (FAB create semantics)', () => {
    const source = readMain();
    // Host object must expose the three callbacks used by content adapters.
    expect(source).toMatch(/host\s*:\s*\{[\s\S]*?\bnavigate\b/);
    expect(source).toMatch(
      /openReadLater\s*:\s*openReadLaterDialog|openReadLater\s*,/,
    );
    expect(source).toMatch(/openCreateNote\s*:/);
    // Create-from-FAB semantics must remain reachable (close panel + navigate + openCreateNote).
    expect(source).toMatch(/function\s+openCreateNoteFromFab|openCreateNoteFromFab\s*=/);
    expect(source).toMatch(/openCreateNoteFromFab|openCreateNote\s*:\s*openCreateNoteFromFab/);
  });

  it('shell is not mounted into the home main content area (#home-view / home hub)', () => {
    const source = readMain();
    expect(source).not.toMatch(
      /mountHomeEntryShell\s*\(\s*(?:homeView|document\.getElementById\(\s*['"]home-view['"]\s*\))/,
    );
    // mountHomeEntryShell must not take the home-hub mount target as its anchor.
    expect(source).not.toMatch(/mountHomeEntryShell\s*\(\s*homeView\b/);
    // Positive: body mount is the orchestration surface.
    expect(source).toMatch(/mountHomeEntryShell\s*\(\s*document\.body\b/);
  });

  it('app.css removes the old independent four-FAB bottom stack offsets', () => {
    const css = readAppCss();
    // Former stack: rl 20 → pt 76 → note 132 → builders 188. Shell owns a single cluster anchor.
    expect(css).not.toMatch(/\.pt-assistant-widget\s*\{[^}]*bottom:\s*76px/);
    expect(css).not.toMatch(/\.note-assistant-widget\s*\{[^}]*bottom:\s*132px/);
    expect(css).not.toMatch(/\.builders-entry\s*\{[^}]*bottom:\s*188px/);
    // Shell cluster remains the single bottom-right stack anchor.
    expect(css).toMatch(/\.home-entry-shell__cluster\s*\{[^}]*bottom:\s*20px/);
  });

  it('main.js wires leave-host / route change to forceRecoverA (overlay must not cross pages)', () => {
    const source = readMain();
    expect(source).toMatch(/forceRecoverA\s*\(/);
    // Leave path should be reachable from route wrapping or an explicit leave helper.
    expect(source).toMatch(
      /forceRecoverA\s*\(\s*['"](?:leave|route|leave-route|leave-host|unmount)['"]|forceRecoverA\s*\(\s*\)/,
    );
  });
});

describe('home-entry-shell unmount force A (T5)', () => {
  /** @type {{ anchor: HTMLElement, shell: ReturnType<typeof mountHomeEntryShell>, entry: { id: string } } | null} */
  let fx = null;

  beforeEach(() => {
    document.body.innerHTML = '';
  });

  afterEach(() => {
    try {
      fx?.shell.unmount();
    } catch {
      // already unmounted in the assertion under test
    }
    fx = null;
    document.body.innerHTML = '';
  });

  it('unmount from C forces A so overlay state does not survive host teardown', async () => {
    fx = mountFixture();
    const { anchor, shell, entry } = fx;

    anchor.querySelector('[data-role="hub"]').click();
    expect(shell.getState().mode).toBe('B');
    await shell.openContent(entry.id);
    expect(shell.getState()).toEqual({ mode: 'C', entryId: entry.id });
    expect(anchor.querySelector('[data-role="overlay"]').hidden).toBe(false);

    shell.unmount();
    expect(shell.getState().mode).toBe('A');
    expect(shell.getState().entryId == null || shell.getState().entryId === undefined).toBe(
      true,
    );
    fx = null;
  });
});
