// @vitest-environment jsdom
/**
 * T6 — four open entries + create entry switch to navigate-to-note.
 * Source + behavioral probes: no residual openDoc-then-hash half-switch.
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { readFrontendJs, readMainSource } from '../helpers/read-frontend-js.js';
import { buildCard } from '../../frontend/src/notes/ui/cards.tsx';
import { state } from '../../frontend/src/host/state.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function read(rel) {
  if (rel === 'frontend/src/boot.ts') return readMainSource();
  if (rel.startsWith('frontend/src/')) return readFrontendJs(rel);
  return readFileSync(join(repoRoot, rel), 'utf8');
}

const apiMocks = vi.hoisted(() => ({
  searchWorkbench: vi.fn(),
}));

vi.mock('../../frontend/src/host/api.ts', () => ({
  searchWorkbench: (...args) => apiMocks.searchWorkbench(...args),
  setImportance: vi.fn(),
  setDone: vi.fn(),
  fetchFileContent: vi.fn(),
}));

vi.mock('../../frontend/src/notes/ui/move-project-dialog.tsx', () => ({
  openMoveProjectDialog: vi.fn(),
  closeMoveProjectDialog: vi.fn(),
  MoveProjectDialog: () => null,
}));

describe('T6 source: open/create entries → navigate-to-note (no modal.display path)', () => {
  const mainJs = read('frontend/src/boot.ts');
  const searchJs = read('frontend/src/notes/ui/search.tsx');
  const cardsJs = read('frontend/src/notes/ui/cards.tsx');
  const assistantJs = read('frontend/src/notes/ui/assistant.tsx');

  it('imports navigateToNote and uses it from cta:open-entry listener', () => {
    expect(mainJs).toMatch(/navigateToNote/);
    expect(mainJs).toMatch(
      /import\s*\{[^}]*navigateToNote[^}]*\}\s*from\s*['"]\.\/router\/index\.(js|ts)['"]/,
    );
    const listenerStart = mainJs.indexOf("addEventListener('cta:open-entry'");
    expect(listenerStart).toBeGreaterThan(-1);
    const listenerSlice = mainJs.slice(listenerStart, listenerStart + 1600);
    expect(listenerSlice).toMatch(/navigateToNote\s*\(/);
    expect(listenerSlice).not.toMatch(/openDoc\s*\(/);
  });

  it('cta:open-entry path does not imply modal.display / half-switch openDoc-then-hash', () => {
    const listenerStart = mainJs.indexOf("addEventListener('cta:open-entry'");
    const listenerSlice = mainJs.slice(listenerStart, listenerStart + 1600);
    expect(listenerSlice).not.toMatch(/modal\.display|display\s*=\s*['"]flex['"]/);
    expect(listenerSlice).not.toMatch(/openDoc[\s\S]{0,200}navigate/);
  });

  it('openCreateNoteFromFab shares micro-route authority (navigate, omit note while creating)', () => {
    const fabStart = mainJs.indexOf('function openCreateNoteFromFab');
    expect(fabStart).toBeGreaterThan(-1);
    const fabSlice = mainJs.slice(fabStart, fabStart + 600);
    expect(fabSlice).toMatch(/navigate\s*\(/);
    expect(fabSlice).toMatch(/#\/workbench/);
    expect(fabSlice).toMatch(/openCreateNote\s*\(/);
    // Must not write a temporary note query while creating
    expect(fabSlice).not.toMatch(/navigateToNote\s*\(/);
  });

  it('workbench-search hit click includes optional layer from data-layer', () => {
    expect(searchJs).toMatch(/dataset\.layer|data-layer/);
    expect(searchJs).toMatch(/detail\.layer/);
    expect(searchJs).toMatch(/cta:open-entry/);
  });

  it('cards / note-assistant still emit cta:open-entry (positioning only)', () => {
    expect(cardsJs).toMatch(/cta:open-entry/);
    expect(assistantJs).toMatch(/cta:open-entry/);
  });

  it('mountWorkbench open branch mounts via openDoc (consumer; not entry-side)', () => {
    const start = mainJs.indexOf('function mountWorkbench');
    expect(start).toBeGreaterThan(-1);
    const slice = mainJs.slice(start, start + 4500);
    expect(slice).toMatch(/openDoc\s*\(/);
  });
});

describe('T6 behavioral: workbench-search layer + cards emit', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    document.body.innerHTML = `
      <div id="gs-wb-wrap" class="gs-search-wrap">
        <input id="gs-wb-input" class="gs-search-input" type="text" autocomplete="off" />
        <div id="gs-wb-dropdown" class="gs-search-dropdown" style="display:none"></div>
      </div>
      <div id="doc-list"></div>
    `;
    const store = {};
    globalThis.localStorage = {
      getItem(key) {
        return Object.prototype.hasOwnProperty.call(store, key) ? store[key] : null;
      },
      setItem(key, value) {
        store[key] = String(value);
      },
      removeItem(key) {
        delete store[key];
      },
    };
    apiMocks.searchWorkbench.mockReset();
    state.index.topicDescriptions = {};
    state.index.diffStatus = new Map();
    state.index.titleCache = new Map();
    state.ui.activeDate = '20260719';
  });

  afterEach(() => {
    vi.useRealTimers();
    document.body.innerHTML = '';
  });

  it('hit click dispatches cta:open-entry with common_path and rendered data-layer', async () => {
    apiMocks.searchWorkbench.mockResolvedValue({
      hits: [{
        title: 'Entry',
        common_path: 'inbox/notes/entry.md',
        layer: 'digest',
        topic: 'demo',
        body: 'snippet body',
      }],
    });

    vi.resetModules();
    const { initWorkbenchSearch } = await import('../../frontend/src/notes/ui/search.tsx');
    initWorkbenchSearch();

    const input = document.getElementById('gs-wb-input');
    input.value = 'entry';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    const handler = vi.fn();
    document.addEventListener('cta:open-entry', handler);

    const hit = document.querySelector('.gs-hit-wb');
    expect(hit).toBeTruthy();
    expect(hit.dataset.layer).toBe('digest');
    hit.click();

    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler.mock.calls[0][0].detail).toEqual({
      common_path: 'inbox/notes/entry.md',
      layer: 'digest',
    });
    document.removeEventListener('cta:open-entry', handler);
  });

  it('hit click without layer still opens (detail may omit layer)', async () => {
    apiMocks.searchWorkbench.mockResolvedValue({
      hits: [{
        title: 'No layer',
        common_path: 'inbox/notes/n.md',
        topic: 'demo',
        body: 'x',
      }],
    });

    vi.resetModules();
    const { initWorkbenchSearch } = await import('../../frontend/src/notes/ui/search.tsx');
    initWorkbenchSearch();

    const input = document.getElementById('gs-wb-input');
    input.value = 'n';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);

    const handler = vi.fn();
    document.addEventListener('cta:open-entry', handler);
    document.querySelector('.gs-hit-wb').click();

    expect(handler).toHaveBeenCalledTimes(1);
    const detail = handler.mock.calls[0][0].detail;
    expect(detail.common_path).toBe('inbox/notes/n.md');
    if (detail.layer !== undefined) {
      expect(detail.layer).toBe('');
    }
    document.removeEventListener('cta:open-entry', handler);
  });

  it('card title / layer badge emit cta:open-entry with optional layer (no openDoc)', () => {
    const entry = {
      common_path: 'inbox/notes/202607191200-a.md',
      created_at: '202607191200',
      layers: ['raw', 'digest'],
    };
    const card = buildCard('id1', entry, 'Title');
    const handler = vi.fn();
    document.addEventListener('cta:open-entry', handler);

    card.querySelector('.doc-title-btn').click();
    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler.mock.calls[0][0].detail).toMatchObject({
      common_path: entry.common_path,
      layer: 'raw',
    });

    handler.mockClear();
    card.querySelector('.badge[data-layer="digest"]').click();
    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler.mock.calls[0][0].detail).toEqual({
      common_path: entry.common_path,
      layer: 'digest',
    });
    document.removeEventListener('cta:open-entry', handler);
  });
});
