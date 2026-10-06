// Node environment — document stub + mocked viewer deps (T5: openDoc/openCreateNote → note outlet)
import { describe, it, expect, beforeEach, vi } from 'vitest';

const { elements, makeEl, locationStub, navigateToNoteMock } = vi.hoisted(() => {
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
      _listeners: {},
      addEventListener(event, fn) {
        if (!this._listeners[event]) this._listeners[event] = [];
        this._listeners[event].push(fn);
      },
      children: [],
      appendChild(child) { this.children.push(child); return child; },
      querySelectorAll: () => [],
      querySelector: () => null,
      focus() {},
      value: '',
      innerHTML: '',
      textContent: '',
      disabled: false,
      scrollTop: 0,
      dataset: {},
      href: '',
      hidden: id === 'note-outlet',
      removeAttribute() {},
      remove() { if (id && elements[id] === this) delete elements[id]; },
      closest: () => null,
    };
    if (id) elements[id] = el;
    return el;
  };

  const locationStub = { hash: '#/spark?date=20260710', href: 'http://localhost/#/spark?date=20260710' };
  const navigateToNoteMock = vi.fn().mockReturnValue(true);

  globalThis.document = {
    getElementById: (id) => (id ? makeEl(id) : null),
    addEventListener: () => {},
    createElement: () => makeEl(),
    createTextNode: (text) => ({ textContent: text }),
    querySelectorAll: () => [],
    querySelector: () => null,
    body: { style: {}, appendChild: () => {} },
    dispatchEvent: vi.fn(),
  };
  globalThis.requestAnimationFrame = (fn) => fn();
  globalThis.marked = undefined;
  globalThis.alert = vi.fn();
  globalThis.location = locationStub;

  return { elements, makeEl, locationStub, navigateToNoteMock };
});

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchFileContent: vi.fn().mockResolvedValue('# Hello note\n\nbody'),
  fetchAnnotation: vi.fn().mockResolvedValue({}),
  saveFile: vi.fn().mockResolvedValue({ ok: true }),
  createNote: vi.fn().mockResolvedValue({
    ok: true,
    id: 'a'.repeat(32),
    common_path: 'inbox/notes/202607101430-hello.md',
  }),
  saveNoteDraft: vi.fn().mockResolvedValue({ ok: true }),
  clearNoteDraft: vi.fn().mockResolvedValue({ ok: true }),
  getNoteDraft: vi.fn().mockResolvedValue({ content: '' }),
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
vi.mock('../../frontend/src/host/constants.ts', () => ({
  notesFileRelPath: (layer, path) => `notes/${layer}/${path}`,
}));
vi.mock('../../frontend/src/shared/mermaid-render.ts', () => ({
  initMermaid: vi.fn(),
  renderMermaidBlocks: vi.fn().mockResolvedValue(undefined),
}));
vi.mock('../../frontend/src/router/index.ts', async () => {
  const actual = await vi.importActual('../../frontend/src/router/index.ts');
  return {
    ...actual,
    navigateToNote: (...args) => navigateToNoteMock(...args),
  };
});

import {
  openCreateNote,
  openDoc,
  closeModal,
  enterEditMode,
  saveDoc,
} from '../../frontend/src/notes/viewer.ts';
import * as api from '../../frontend/src/host/api.ts';
import { state } from '../../frontend/src/host/state.ts';

function resetDom() {
  for (const id of [
    'note-outlet', 'md-modal', 'md-body', 'md-edit-area', 'md-panel-title', 'md-close', 'md-backdrop',
    'date-heading', 'doc-list',
    'comment-dialog', 'btn-edit', 'btn-add-comment', 'btn-save',
    'btn-cancel-edit', 'md-lang-bar', 'md-file-size',
    'md-links-bar', 'md-tags-bar',
    'btn-copy-path', 'btn-open-in-chat', 'btn-goto-kb',
    'comment-float-nav',
  ]) {
    const el = makeEl(id);
    el.style.display = id === 'md-edit-area' || id === 'btn-save' || id === 'btn-cancel-edit'
      || id === 'md-lang-bar'
      ? 'none'
      : '';
    el.value = '';
    el.textContent = '';
    el.innerHTML = '';
    el.classList._set.clear();
    el.dataset = {};
    if (id === 'note-outlet') el.hidden = true;
  }
  makeEl('md-modal').style.display = 'none';
  document.body.style.overflow = '';
  locationStub.hash = '#/spark?date=20260710';
  state.ui.activeDate = '20260710';
  state.viewer.entry = null;
  state.viewer.rawText = '';
  state.viewer.createSession = null;
  vi.clearAllMocks();
  api.createNote.mockResolvedValue({
    ok: true,
    id: 'a'.repeat(32),
    common_path: 'inbox/notes/202607101430-hello.md',
  });
  api.saveNoteDraft.mockResolvedValue({ ok: true });
  api.clearNoteDraft.mockResolvedValue({ ok: true });
  api.getNoteDraft.mockResolvedValue({ content: '' });
  api.fetchFileContent.mockResolvedValue('# Hello note\n\nbody');
  api.saveFile.mockResolvedValue({ ok: true });
  navigateToNoteMock.mockReturnValue(true);
  globalThis.alert.mockClear();
}

describe('T5 openDoc mounts into note outlet (not md-modal flex)', () => {
  beforeEach(() => {
    resetDom();
  });

  it('openDoc reveals #note-outlet and keeps #md-modal display none', async () => {
    const entry = {
      common_path: 'inbox/notes/demo.md',
      created_at: '20260710120000',
      translations: {},
    };
    state.index.filteredGroups = [
      { date: '20260710', entries: new Array(6).fill(null) },
    ];

    await openDoc(entry, 'raw');

    expect(makeEl('note-outlet').hidden).toBe(false);
    expect(makeEl('md-modal').style.display).not.toBe('flex');
    expect(makeEl('md-modal').style.display).toBe('none');
    expect(makeEl('md-panel-title').textContent).toBe('Jul 10, 2026 (Fri)  ·  6 items');
    expect(makeEl('date-heading').style.display).toBe('none');
    expect(makeEl('doc-list').style.display).toBe('none');
    expect(state.viewer.entry).toEqual(entry);
  });

  it('enterEditMode / saveDoc bind the same chrome tree ids after openDoc', async () => {
    const entry = {
      common_path: 'inbox/notes/edit-me.md',
      created_at: '20260710120000',
      translations: {},
    };
    await openDoc(entry, 'raw');

    enterEditMode();
    expect(makeEl('md-edit-area').style.display).not.toBe('none');
    expect(makeEl('btn-save').style.display).not.toBe('none');
    expect(makeEl('md-body').style.display).toBe('none');

    makeEl('md-edit-area').value = '# Edited\n';
    await saveDoc();

    expect(api.saveFile).toHaveBeenCalled();
    expect(makeEl('note-outlet').hidden).toBe(false);
    expect(makeEl('md-modal').style.display).toBe('none');
  });
});

describe('T5 openCreateNote / finalizeCreateSession on note outlet', () => {
  beforeEach(() => {
    resetDom();
  });

  it('create session is outlet-local: shows #note-outlet, omits note in location', async () => {
    await openCreateNote({ temp_id: 'tmp-create' });

    expect(makeEl('note-outlet').hidden).toBe(false);
    expect(makeEl('note-outlet').dataset.wbMode).toBe('create');
    expect(makeEl('md-modal').style.display).toBe('none');
    expect(makeEl('md-modal').style.display).not.toBe('flex');
    expect(state.viewer.createSession?.status).toBe('creating');
    expect(locationStub.hash).not.toMatch(/[?&]note=/);
    expect(locationStub.hash).toBe('#/spark?date=20260710');
  });

  it('create success: navigate note=common_path from createNote', async () => {
    await openCreateNote({ temp_id: 'tmp-ok' });
    makeEl('md-edit-area').value = 'hello archived note';
    vi.clearAllMocks();
    api.createNote.mockResolvedValue({
      ok: true,
      id: 'b'.repeat(32),
      common_path: 'inbox/notes/202607101431-body.md',
    });
    api.clearNoteDraft.mockResolvedValue({ ok: true });
    navigateToNoteMock.mockReturnValue(true);

    await closeModal();

    expect(api.createNote).toHaveBeenCalledWith({
      body: 'hello archived note',
      source_type: 'jot',
    });
    expect(navigateToNoteMock).toHaveBeenCalledWith({
      date: '20260710',
      note: 'inbox/notes/202607101431-body.md',
    });
    expect(state.viewer.createSession).toBeNull();
    expect(makeEl('md-modal').style.display).toBe('none');
  });

  it('create failure: visible error, stay on outlet create state, no Dialog / md-modal fallback', async () => {
    await openCreateNote({ temp_id: 'tmp-fail' });
    makeEl('md-edit-area').value = 'will fail';
    api.createNote.mockRejectedValueOnce(new Error('409: conflict'));

    await closeModal();

    expect(globalThis.alert).toHaveBeenCalledWith(expect.stringContaining('409: conflict'));
    expect(state.viewer.createSession?.tempId).toBe('tmp-fail');
    expect(state.viewer.createSession?.status).toBe('creating');
    expect(makeEl('note-outlet').hidden).toBe(false);
    expect(makeEl('md-modal').style.display).toBe('none');
    expect(makeEl('md-modal').style.display).not.toBe('flex');
    expect(navigateToNoteMock).not.toHaveBeenCalled();
    expect(locationStub.hash).not.toMatch(/[?&]note=/);
  });
});
