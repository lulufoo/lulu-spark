// Node environment — uses global document stub (same pattern as viewer-badge.test.js)
import { describe, it, expect, beforeEach, vi } from 'vitest';

const { elements, makeEl, clickHandlers } = vi.hoisted(() => {
  const elements = {};
  const clickHandlers = {};

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
      addEventListener(type, handler) {
        if (type === 'click' && id) clickHandlers[id] = handler;
      },
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

  return { elements, makeEl, clickHandlers };
});

vi.mock('../frontend/js/host/api.js', () => ({
  fetchFileContent: vi.fn(),
  fetchAnnotation: vi.fn(),
  saveFile: vi.fn(),
  fetchKbStatus: vi.fn().mockResolvedValue({ error: null, total: 0, ahead: 0 }),
}));
vi.mock('../frontend/js/host/state.js', async () => {
  const actual = await vi.importActual('../frontend/js/host/state.js');
  return actual;
});
vi.mock('../frontend/js/corpus/kb-comments.js', () => ({
  renderKbComments: vi.fn(),
  initKbCommentEvents: vi.fn(),
}));
vi.mock('../frontend/js/corpus/kb-highlights.js', () => ({
  applyKbHighlights: vi.fn(),
  initKbHighlightUI: vi.fn(),
}));
vi.mock('../frontend/js/corpus/kb-links-bar.js', () => ({ renderKbLinksBar: vi.fn() }));

import { state } from '../frontend/js/host/state.js';
import { saveKbDoc } from '../frontend/js/corpus/kb-viewer.js';

function enterKbEditMode() {
  clickHandlers['kb-btn-edit']();
}

describe('_kbEnterEditMode (via #kb-btn-edit click)', () => {
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

  it('registers click handler on #kb-btn-edit', () => {
    expect(clickHandlers['kb-btn-edit']).toBeTypeOf('function');
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
