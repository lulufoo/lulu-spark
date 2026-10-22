// @vitest-environment jsdom
/**
 * T6 — four open entries + create entry switch to navigate-to-note.
 * Source + behavioral probes: no residual openDoc-then-hash half-switch.
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { readFrontendJs, readMainSource } from '../helpers/read-frontend-js.js';
import { buildCard } from '../../frontend/src/notes/ui/cards.tsx';
import { state } from '../../frontend/src/host/state.ts';
import { NotesSearchModal } from '../../frontend/src/notes/ui/search.tsx';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function read(rel) {
  if (rel === 'frontend/src/boot.ts') return readMainSource();
  if (rel.startsWith('frontend/src/')) return readFrontendJs(rel);
  return readFileSync(join(repoRoot, rel), 'utf8');
}

const apiMocks = vi.hoisted(() => ({
  searchSpark: vi.fn(),
}));

vi.mock('../../frontend/src/host/api.ts', () => ({
  searchSpark: (...args) => apiMocks.searchSpark(...args),
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
  const modalJs = read('frontend/src/shared/search-modal/search-modal.tsx');
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
    expect(listenerSlice).toMatch(/enterDocProject\s*\(/);
    expect(listenerSlice).toMatch(/navigateToNote\s*\(/);
    expect(listenerSlice).not.toMatch(/openDoc\s*\(/);
  });

  it('cta:open-entry path does not imply modal.display / half-switch openDoc-then-hash', () => {
    const listenerStart = mainJs.indexOf("addEventListener('cta:open-entry'");
    const listenerSlice = mainJs.slice(listenerStart, listenerStart + 1600);
    expect(listenerSlice).not.toMatch(/modal\.display|display\s*=\s*['"]flex['"]/);
    expect(listenerSlice).not.toMatch(/openDoc[\s\S]{0,200}navigate/);
  });

  it('hub create-note FAB path is retired (no openCreateNoteFromFab)', () => {
    expect(mainJs).not.toMatch(/function openCreateNoteFromFab/);
    expect(mainJs).not.toMatch(/openCreateNoteFromFab/);
  });

  it('search modal opens a hit through cta:open-entry without layer, and the notes page does not override it', () => {
    const openStart = modalJs.indexOf('function openEntry');
    expect(openStart).toBeGreaterThan(-1);
    const openSlice = modalJs.slice(openStart, openStart + 200);
    expect(openSlice).toMatch(/cta:open-entry/);
    expect(openSlice).not.toMatch(/layer/);
    expect(modalJs).not.toMatch(/gs-hit-layer/);
    expect(searchJs).not.toMatch(/onOpenDoc/);
  });

  it('cards / note-assistant still emit cta:open-entry (positioning only)', () => {
    expect(cardsJs).toMatch(/cta:open-entry/);
    expect(assistantJs).toMatch(/cta:open-entry/);
  });

  it('mountSpark open branch mounts via openDoc (consumer; not entry-side)', () => {
    const start = mainJs.indexOf('function mountSpark');
    expect(start).toBeGreaterThan(-1);
    const slice = mainJs.slice(start, start + 4500);
    expect(slice).toMatch(/openDoc\s*\(/);
  });
});

describe('T6 behavioral: main-layer open (spark-search hits + card title)', () => {
  let container;
  let root;

  beforeEach(() => {
    vi.useFakeTimers();
    document.body.innerHTML = '<div id="doc-list"></div>';
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
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
    apiMocks.searchSpark.mockReset();
    state.index.topicDescriptions = {};
    state.index.titleCache = new Map();
    state.ui.activeDate = '20260719';
  });

  afterEach(() => {
    act(() => root.unmount());
    vi.useRealTimers();
    document.body.innerHTML = '';
  });

  async function searchAndGetFirstHit(q) {
    act(() => {
      root.render(createElement(NotesSearchModal, { open: true, topicTitles: {}, onClose: () => {} }));
    });
    const input = container.querySelector('[data-role="search-modal-input"]');
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(input, q);
      input.dispatchEvent(new Event('input', { bubbles: true }));
      await vi.advanceTimersByTimeAsync(300);
    });
    return container.querySelector('[data-role="search-hit"]');
  }

  it('hit click dispatches cta:open-entry with id only (no layer passthrough)', async () => {
    apiMocks.searchSpark.mockResolvedValue({
      hits: [{
        title: 'Entry',
        id: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
        layer: 'digest',
        project_id: 'demo',
        body: 'snippet body',
      }],
    });

    const hit = await searchAndGetFirstHit('entry');
    expect(hit).toBeTruthy();
    expect(document.querySelector('.gs-hit-layer')).toBeNull();

    const handler = vi.fn();
    document.addEventListener('cta:open-entry', handler);
    act(() => hit.click());

    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler.mock.calls[0][0].detail).toEqual({
      id: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
    });
    expect('layer' in handler.mock.calls[0][0].detail).toBe(false);
    document.removeEventListener('cta:open-entry', handler);
  });

  it('hit click without layer still opens (detail may omit layer)', async () => {
    apiMocks.searchSpark.mockResolvedValue({
      hits: [{
        title: 'No layer',
        id: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
        project_id: 'demo',
        body: 'x',
      }],
    });

    const hit = await searchAndGetFirstHit('n');
    expect(hit).toBeTruthy();

    const handler = vi.fn();
    document.addEventListener('cta:open-entry', handler);
    act(() => hit.click());

    expect(handler).toHaveBeenCalledTimes(1);
    const detail = handler.mock.calls[0][0].detail;
    expect(detail.id).toBe('bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb');
    if (detail.layer !== undefined) {
      expect(detail.layer).toBe('');
    }
    document.removeEventListener('cta:open-entry', handler);
  });

  it('card title click emits cta:open-entry with main layer raw (layer badges removed)', () => {
    const entry = {
      project_id: 'inbox',
      created_at: '202607191200',
      layers: ['raw', 'digest'],
    };
    const card = buildCard('id1', entry, 'Title');
    const handler = vi.fn();
    document.addEventListener('cta:open-entry', handler);

    expect(card.querySelector('.badge[data-layer]')).toBeNull();

    card.querySelector('.doc-title-btn').click();
    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler.mock.calls[0][0].detail).toEqual({
      id: 'id1',
      layer: 'raw',
    });
    document.removeEventListener('cta:open-entry', handler);
  });

  it('digest-only entry: title click emits cta:open-entry falling back to layer digest', () => {
    const entry = {
      project_id: 'inbox',
      created_at: '202607191300',
      layers: ['digest'],
    };
    const card = buildCard('id2', entry, 'Title');
    const handler = vi.fn();
    document.addEventListener('cta:open-entry', handler);

    card.querySelector('.doc-title-btn').click();
    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler.mock.calls[0][0].detail).toEqual({
      id: 'id2',
      layer: 'digest',
    });
    document.removeEventListener('cta:open-entry', handler);
  });
});
