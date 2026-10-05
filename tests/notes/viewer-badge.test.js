// Node environment — uses global document stub (same pattern as sidebar.test.js)
import { describe, it, expect, beforeEach, vi } from 'vitest';

const { makeEl } = vi.hoisted(() => {
  const elements = {};
  const makeEl = (id = '') => {
    if (id && elements[id]) return elements[id];
    const el = {
      id,
      style: {},
      classList: {
        _set: new Set(),
        contains(cls) { return this._set.has(cls); },
        add(cls) { this._set.add(cls); },
        remove(cls) { this._set.delete(cls); },
        toggle(cls, force) {
          if (force === true) this._set.add(cls);
          else if (force === false) this._set.delete(cls);
          else if (this._set.has(cls)) this._set.delete(cls); else this._set.add(cls);
        },
      },
      addEventListener: () => {},
      children: [],
      appendChild(child) { this.children.push(child); return child; },
      querySelectorAll: () => [],
      focus() {},
      value: '',
      innerHTML: '',
      textContent: '',
      disabled: false,
      scrollTop: 0,
      dataset: {},
      href: '',
      removeAttribute() {},
      remove() {
        if (id && elements[id] === this) delete elements[id];
      },
    };
    if (id) elements[id] = el;
    return el;
  };

  globalThis.document = {
    getElementById: (id) => (id ? makeEl(id) : null),
    addEventListener: () => {},
    createElement: () => makeEl(),
    createTextNode: (text) => ({ textContent: text }),
    querySelectorAll: () => [],
    querySelector: () => null,
    body: { style: {}, appendChild: () => {}, insertAdjacentHTML: () => {} },
    dispatchEvent: () => {},
  };

  globalThis.window = {
    location: { hash: '', replace() {} },
    history: { back() {}, length: 1 },
    addEventListener() {},
  };
  globalThis.requestAnimationFrame = (fn) => fn();
  globalThis.marked = undefined;

  return { makeEl };
});

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchFileContent: vi.fn().mockResolvedValue('# Test'),
  fetchAnnotation: vi.fn().mockResolvedValue({}),
  saveFile: vi.fn(),
  commitFiles: vi.fn(),
}));
vi.mock('../../frontend/src/host/state.ts', async () => {
  const actual = await vi.importActual('../../frontend/src/host/state.ts');
  return actual;
});
vi.mock('../../frontend/src/notes/ui/cards.tsx', () => ({
  updateTitlesInDOM: vi.fn(),
  updateDiffInDOM: vi.fn(),
}));
vi.mock('../../frontend/src/island.ts', () => ({
  renderToHtml: () => '',
}));
vi.mock('../../frontend/src/notes/ui/links-bar.tsx', () => ({ renderLinksBar: vi.fn() }));
vi.mock('../../frontend/src/notes/ui/tags-bar.tsx', () => ({ renderTagsBar: vi.fn() }));
vi.mock('../../frontend/src/notes/ui/comments.tsx', () => ({ renderComments: vi.fn() }));
vi.mock('../../frontend/src/notes/ui/delete-dialog.tsx', () => ({ openDeleteDialog: vi.fn() }));
vi.mock('../../frontend/src/doc-editor/highlights.ts', () => ({
  applyCachedHighlights: vi.fn(),
  initDocHighlightOverlay: vi.fn(),
  cleanupDocHighlightOverlay: vi.fn(),
}));
vi.mock('../../frontend/src/knowledge/viewer.ts', () => ({
  openKbDoc: vi.fn(),
  saveKbDoc: vi.fn(),
}));
vi.mock('../../frontend/src/knowledge/ui/knowledge-search.tsx', () => ({
  mountKnowledgeSearch: vi.fn(),
  triggerKnowledgeSearch: vi.fn(),
}));
vi.mock('../../frontend/src/host/constants.ts', () => ({
  notesFileRelPath: (layer, path) => `notes/${layer}/${path}`,
}));

import { enterEditMode, exitEditMode } from '../../frontend/src/notes/viewer.ts';
import { state } from '../../frontend/src/host/state.ts';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

beforeEach(() => {
  state.viewer.entry = null;
  state.viewer.layer = 'raw';
  state.viewer.rawText = '';
  state.viewer.isKb = false;
});

describe('notes viewer pending commit', () => {
  it('does not keep a reader Pending commit button', () => {
    const page = readFileSync(
      join(dirname(fileURLToPath(import.meta.url)), '../../frontend/src/notes/page.tsx'),
      'utf8',
    );
    expect(page).not.toContain('btn-panel-commit');
    expect(page).not.toContain('Pending commit');
  });
});

describe('enterEditMode', () => {
  it('resets #md-edit-area scrollTop to 0', () => {
    const editArea = makeEl('md-edit-area');
    editArea.scrollTop = 500;
    state.viewer.rawText = 'hello';
    enterEditMode();
    expect(editArea.scrollTop).toBe(0);
  });

  it('resets scrollTop to 0 after focus would scroll to bottom (long text)', () => {
    const editArea = makeEl('md-edit-area');
    editArea.setSelectionRange = vi.fn();
    editArea.focus = vi.fn(function focusMock() {
      this.scrollTop = 9999;
    });
    state.viewer.rawText = Array.from({ length: 60 }, (_, i) => `line ${i + 1}`).join('\n');
    editArea.scrollTop = 500;
    enterEditMode();
    expect(editArea.scrollTop).toBe(0);
    expect(editArea.focus).toHaveBeenCalled();
  });
});

describe('exitEditMode', () => {
  it('does not throw when viewer entry is null', () => {
    state.viewer.entry = null;
    expect(() => exitEditMode(false)).not.toThrow();
  });
});
