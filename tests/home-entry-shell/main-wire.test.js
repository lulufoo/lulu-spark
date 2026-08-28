// @vitest-environment jsdom
/**
 * T5: main.js once-mount wiring + host callbacks + leave/unmount force A;
 * remove old four-FAB mutex / CSS stack.
 */
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { createContentRegistry } from '../../frontend/src/home-entry-shell/content-registry.ts';
import { mountHomeEntryShell } from '../../frontend/src/home-entry-shell/shell.tsx';
import { readFrontendJs, readMainSource } from '../helpers/read-frontend-js.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function readMain() {
  return readMainSource();
}

function readAppCss() {
  return readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');
}

/** Count top-level mount*AssistantWidget(document.body …) calls in main.js. */
function countLegacyBodyMounts(source) {
  const re =
    /mount(?:ReadLater|TodoTask|Note|Builders)AssistantWidget\s*\(\s*document\.body\b/g;
  return (source.match(re) || []).length;
}

/** Detect the retired document capture-phase FAB mutex listener. */
function hasFabMutexListener(source) {
  return (
    /closest\(\s*['"]\.rl-assistant-fab['"]\s*\)/.test(source) &&
    /closest\(\s*['"]\.todo-assistant-fab['"]\s*\)/.test(source) &&
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
    expect(css).not.toMatch(/\.todo-assistant-widget\s*\{[^}]*bottom:\s*76px/);
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

  it('main.js listens ai-assistant:opened; Present surface → Home, not the FAB overlay', () => {
    const source = readMain();
    expect(source).toMatch(/ai-assistant:opened/);
    expect(source).toMatch(/surface/);
    expect(source).toMatch(/['"]Present['"]/);
    expect(source).toMatch(/navigate\('#\/home'\)/);
    expect(source).not.toMatch(/handleAiAssistantOpenedPayload[\s\S]{0,400}presentNormalize/);
    expect(source).toMatch(/pending/i);
    expect(source).not.toMatch(/setFocus|set_focus/);
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

describe('home-entry-shell workbench Binding (t2)', () => {
  it('main.js Sets workbench Binding once immediately after mountHomeEntryShell', () => {
    const source = readMain();
    expect(source).toMatch(
      /(?:const\s+)?homeEntryShell\s*=\s*mountHomeEntryShell\s*\(\s*document\.body[\s\S]*?\}\s*\)\s*;[\s\S]*?void\s+setWorkbenchBinding\s*\(\s*\)/,
    );
    const calls = source.match(/void\s+setWorkbenchBinding\s*\(\s*\)/g) || [];
    expect(calls.length, 'exactly one process-level workbench Set').toBe(1);
    expect(source).toMatch(/setWorkbenchBinding/);
  });

  it('does not put the Set in shell.js, #/home enter/leave, or Host process start', () => {
    const source = readMain();
    const shellJs = readFrontendJs('frontend/src/home-entry-shell/shell.tsx');
    const libRs = readFileSync(join(repoRoot, 'src-tauri/src/lib.rs'), 'utf8');
    expect(shellJs).not.toMatch(/setWorkbenchBinding|set_binding|reset_binding/);
    expect(source).not.toMatch(
      /#\/home[\s\S]{0,160}setWorkbenchBinding|setWorkbenchBinding[\s\S]{0,160}#\/home/,
    );
    expect(libRs).not.toMatch(/setWorkbenchBinding/);
    const setupIdx = libRs.indexOf('.setup(|app|');
    expect(setupIdx).toBeGreaterThanOrEqual(0);
    const setupSlice = libRs.slice(setupIdx, setupIdx + 800);
    expect(setupSlice).not.toMatch(/set_binding/);
  });

  it('business pages have zero Set/Reset: todos, notes, home, selectDate, mountWorkbench', () => {
    const source = readMain();
    const sidebarJs = readFrontendJs('frontend/src/notes/ui/sidebar.tsx');
    const sidebarCommandsJs = readFrontendJs('frontend/src/notes/commands/sidebar.ts');
    const lifeJs = readFrontendJs('frontend/src/todo-task/commands/lifecycle.ts');
    expect(source).not.toMatch(
      /\bbuildNotesBinding\b|\bresetNotesBinding\b|\bbuildTodosBinding\b|\bresetTodosBinding\b/,
    );
    const mountWorkbenchIdx = source.indexOf('function mountWorkbench');
    expect(mountWorkbenchIdx).toBeGreaterThanOrEqual(0);
    const mountSlice = source.slice(mountWorkbenchIdx, mountWorkbenchIdx + 400);
    expect(mountSlice).not.toMatch(/set_binding|reset_binding|setWorkbenchBinding|buildNotesBinding/);

    const mountTodosIdx = source.indexOf('function mountTodoTasksRoute');
    expect(mountTodosIdx).toBeGreaterThanOrEqual(0);
    const todosSlice = source.slice(mountTodosIdx, mountTodosIdx + 500);
    expect(todosSlice).not.toMatch(/resetNotesBinding|set_binding|reset_binding/);

    expect(sidebarJs).not.toMatch(/buildNotesBinding|set_binding|reset_binding|setWorkbenchBinding/);
    expect(sidebarCommandsJs).not.toMatch(
      /buildNotesBinding|set_binding|reset_binding|setWorkbenchBinding/,
    );
    expect(lifeJs).not.toMatch(/buildTodosBinding|resetTodosBinding|set_binding|reset_binding/);
  });
});

