// Node environment — uses global document stub (same pattern as viewer-badge.test.js)
import { describe, it, expect, beforeEach, vi } from 'vitest';

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
      addEventListener() {},
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

  globalThis.requestAnimationFrame = (fn) => fn();
  globalThis.marked = undefined;

  return { elements, makeEl };
});

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchFileContent: vi.fn(),
  fetchAnnotation: vi.fn(),
  saveFile: vi.fn(),
}));
vi.mock('../../frontend/src/host/state.ts', async () => {
  const actual = await vi.importActual('../../frontend/src/host/state.ts');
  return actual;
});
vi.mock('../../frontend/src/knowledge/ui/comments.tsx', () => ({
  renderKbComments: vi.fn(),
  initKbComments: vi.fn(),
  cleanupKbComments: vi.fn(),
  initKbCommentEvents: vi.fn(),
  KbCommentsBar: () => null,
  KbCommentFloatNav: () => null,
}));
vi.mock('../../frontend/src/doc-editor/highlights.ts', () => ({
  applyCachedHighlights: vi.fn(),
  initDocHighlightOverlay: vi.fn(),
  cleanupDocHighlightOverlay: vi.fn(),
}));
vi.mock('../../frontend/src/knowledge/ui/links-bar.tsx', () => ({
  renderKbLinksBar: vi.fn(),
  KbLinksBar: () => null,
}));

import { state } from '../../frontend/src/host/state.ts';
import { saveKbDoc, _kbEnterEditMode } from '../../frontend/src/knowledge/viewer.ts';

function enterKbEditMode() {
  _kbEnterEditMode();
}

describe('_kbEnterEditMode (via command, not import-time #kb-btn-edit)', () => {
  beforeEach(() => {
    for (const el of Object.values(elements)) {
      el.style = {};
      el.scrollTop = 0;
      el.value = '';
      el.focus = () => {};
      delete el.setSelectionRange;
    }
    state.viewer.rawText = '';
  });

  it('exports _kbEnterEditMode as a command', () => {
    expect(_kbEnterEditMode).toBeTypeOf('function');
  });

  it('resets #kb-md-edit-area scrollTop to 0', () => {
    const editArea = makeEl('kb-md-edit-area');
    editArea.scrollTop = 500;
    state.viewer.rawText = 'hello';
    enterKbEditMode();
    expect(editArea.scrollTop).toBe(0);
  });

  it('resets scrollTop to 0 after focus would scroll to bottom (long text)', () => {
    const editArea = makeEl('kb-md-edit-area');
    editArea.setSelectionRange = vi.fn();
    editArea.focus = vi.fn(function focusMock() {
      this.scrollTop = 9999;
    });
    state.viewer.rawText = Array.from({ length: 60 }, (_, i) => `line ${i + 1}`).join('\n');
    editArea.scrollTop = 500;
    enterKbEditMode();
    expect(editArea.scrollTop).toBe(0);
    expect(editArea.focus).toHaveBeenCalled();
  });

  it('preserves DOM switch: shows edit area, hides body, toggles buttons', () => {
    const body = makeEl('kb-md-body');
    const editArea = makeEl('kb-md-edit-area');
    const btnSave = makeEl('kb-btn-save');
    const btnCancel = makeEl('kb-btn-cancel-edit');
    const btnEdit = makeEl('kb-btn-edit');
    body.style.display = '';
    editArea.style.display = 'none';
    btnSave.style.display = 'none';
    btnCancel.style.display = 'none';
    btnEdit.style.display = '';
    state.viewer.rawText = 'content';
    enterKbEditMode();
    expect(body.style.display).toBe('none');
    expect(editArea.style.display).toBe('');
    expect(editArea.value).toBe('content');
    expect(btnSave.style.display).toBe('');
    expect(btnCancel.style.display).toBe('');
    expect(btnEdit.style.display).toBe('none');
  });

  it('does not affect saveKbDoc export', () => {
    expect(saveKbDoc).toBeTypeOf('function');
  });
});
