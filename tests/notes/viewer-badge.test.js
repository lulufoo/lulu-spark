// Node environment — uses global document stub (same pattern as sidebar.test.js)
import { describe, it, expect, beforeEach, vi } from 'vitest';

// ── Global document stub ──────────────────────────────────────────────────
// Must be set up BEFORE importing viewer.js (module-level addEventListener calls).
// vi.hoisted runs before ESM imports in vitest node mode.

const { elements, makeEl } = vi.hoisted(() => {
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
  globalThis.marked = undefined; // not defined → renderDocBody falls back to <pre>

  return { elements, makeEl };
});

// ── Mock all viewer.js dependencies ──────────────────────────────────────

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
vi.mock('../../frontend/src/notes/cards.tsx', () => ({
  updateTitlesInDOM: vi.fn(),
  updateDiffInDOM: vi.fn(),
}));
vi.mock('../../frontend/src/island.ts', () => ({
  renderToHtml: () => '',
}));
vi.mock('../../frontend/src/notes/links-bar.tsx', () => ({ renderLinksBar: vi.fn() }));
vi.mock('../../frontend/src/notes/tags-bar.tsx', () => ({ renderTagsBar: vi.fn() }));

vi.mock('../../frontend/src/notes/comments.tsx', () => ({ renderComments: vi.fn() }));
vi.mock('../../frontend/src/notes/delete-dialog.tsx', () => ({ openDeleteDialog: vi.fn() }));
vi.mock('../../frontend/src/doc-editor/highlights.ts', () => ({
  applyCachedHighlights: vi.fn(),
  initDocHighlightOverlay: vi.fn(),
  cleanupDocHighlightOverlay: vi.fn(),
}));
vi.mock('../../frontend/src/corpus/corpus-viewer.ts', () => ({
  openKbDoc: vi.fn(),
  saveKbDoc: vi.fn(),
}));
vi.mock('../../frontend/src/corpus/corpus-knowledge-search.tsx', () => ({
  mountKnowledgeSearch: vi.fn(),
  triggerKnowledgeSearch: vi.fn(),
}));
vi.mock('../../frontend/src/host/constants.ts', () => ({
  getGithubUserUrl: vi.fn(() => ''),
  workbenchGithubBlobBase: vi.fn(() => null),
}));

import {
  showPendingBadge,
  hidePendingBadge,
  enterEditMode,
  exitEditMode,
  closeModal,
  openDoc,
} from '../../frontend/src/notes/viewer.ts';
import { state } from '../../frontend/src/host/state.ts';

// ── Test helpers ──────────────────────────────────────────────────────────

function badge() { return makeEl('btn-panel-commit'); }

const makeEntry = (commonPath = 'ai/note.md') => ({
  common_path: commonPath,
  created_at: '202601011200',
  layers: ['raw'],
  translations: {},
});

beforeEach(() => {
  badge().style.display = 'none';
  makeEl('md-commit-dialog').classList.remove('open');
  state.viewer.entry = null;
  state.viewer.layer = 'raw';
  state.viewer.rawText = '';
  state.viewer.isKb = false;
  state.index.diffStatus = new Map();
  state.ui.workbenchKnowledgeRoot = '';
  state.index.topicRepos = {};
  state.index.titleCache = new Map();
});

// ── showPendingBadge / hidePendingBadge ───────────────────────────────────

describe('showPendingBadge', () => {
  it('makes #btn-panel-commit visible', () => {
    showPendingBadge();
    expect(badge().style.display).toBe('');
  });

  it('does not throw when #btn-panel-commit is absent', () => {
    const saved = elements['btn-panel-commit'];
    delete elements['btn-panel-commit'];
    expect(() => showPendingBadge()).not.toThrow();
    if (saved) elements['btn-panel-commit'] = saved;
  });
});

describe('hidePendingBadge', () => {
  it('hides #btn-panel-commit', () => {
    badge().style.display = '';
    hidePendingBadge();
    expect(badge().style.display).toBe('none');
  });
});

// ── enterEditMode ─────────────────────────────────────────────────────────

describe('enterEditMode', () => {
  it('hides #btn-panel-commit', () => {
    badge().style.display = '';
    enterEditMode();
    expect(badge().style.display).toBe('none');
  });

  it('closes md-commit-dialog if it was open', () => {
    makeEl('md-commit-dialog').classList.add('open');
    enterEditMode();
    expect(makeEl('md-commit-dialog').classList.contains('open')).toBe(false);
  });

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

// ── exitEditMode ──────────────────────────────────────────────────────────

describe('exitEditMode', () => {
  it('shows badge when diffStatus has entry', () => {
    const entry = makeEntry('ai/note.md');
    state.viewer.entry = entry;
    state.viewer.layer = 'raw';
    state.index.diffStatus.set('raw/ai/note.md', true);
    exitEditMode(false);
    expect(badge().style.display).toBe('');
  });

  it('hides badge when diffStatus has no entry', () => {
    const entry = makeEntry('ai/note.md');
    state.viewer.entry = entry;
    state.viewer.layer = 'raw';
    state.index.diffStatus = new Map();
    exitEditMode(false);
    expect(badge().style.display).toBe('none');
  });

  it('hides badge (no crash) when viewer entry is null', () => {
    state.viewer.entry = null;
    expect(() => exitEditMode(false)).not.toThrow();
    expect(badge().style.display).toBe('none');
  });
});

// ── closeModal ────────────────────────────────────────────────────────────

describe('closeModal', () => {
  it('hides badge after close', () => {
    badge().style.display = '';
    closeModal();
    expect(badge().style.display).toBe('none');
  });
});

// ── openDoc ───────────────────────────────────────────────────────────────

describe('openDoc', () => {
  it('shows badge when hasDiff is true', async () => {
    const entry = makeEntry('ai/note.md');
    state.index.diffStatus.set('raw/ai/note.md', true);
    await openDoc(entry, 'raw');
    expect(badge().style.display).toBe('');
  });

  it('hides badge when hasDiff is false', async () => {
    const entry = makeEntry('ai/note.md');
    state.index.diffStatus = new Map();
    await openDoc(entry, 'raw');
    expect(badge().style.display).toBe('none');
  });
});
