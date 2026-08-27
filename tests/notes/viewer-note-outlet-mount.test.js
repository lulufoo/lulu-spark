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

  const locationStub = { hash: '#/workbench?date=20260710', href: 'http://localhost/#/workbench?date=20260710' };
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

vi.mock('../../frontend/js/host/api.js', () => ({
  fetchFileContent: vi.fn().mockResolvedValue('# Hello note\n\nbody'),
  fetchAnnotation: vi.fn().mockResolvedValue({}),
  fetchDiffStatus: vi.fn().mockResolvedValue({
    new: [], modified: [], deleted: [], renamed: [], conflicted: [], total: 0, ahead: 0,
  }),
  saveFile: vi.fn().mockResolvedValue({ ok: true }),
  commitFiles: vi.fn().mockResolvedValue({ ok: true }),
  revertFile: vi.fn().mockResolvedValue({ ok: true }),
  archiveDocument: vi.fn().mockResolvedValue({
    ok: true,
    id: 'a'.repeat(32),
    common_path: 'inbox/notes/202607101430-hello.md',
  }),
  saveNoteDraft: vi.fn().mockResolvedValue({ ok: true }),
  clearNoteDraft: vi.fn().mockResolvedValue({ ok: true }),
  getNoteDraft: vi.fn().mockResolvedValue({ content: '' }),
}));
vi.mock('../../frontend/js/host/state.js', async () => {
  const actual = await vi.importActual('../../frontend/js/host/state.js');
  return { ...actual, loadDiffStatus: vi.fn().mockResolvedValue(undefined) };
});
vi.mock('../../frontend/js/notes/cards.js', () => ({
  updateTitlesInDOM: vi.fn(),
  updateDiffInDOM: vi.fn(),
}));
vi.mock('../../frontend/js/notes/links-bar.js', () => ({ renderLinksBar: vi.fn() }));
vi.mock('../../frontend/js/notes/tags-bar.js', () => ({ renderTagsBar: vi.fn() }));
vi.mock('../../frontend/js/notes/comments.js', () => ({ renderComments: vi.fn() }));
vi.mock('../../frontend/js/notes/delete-dialog.js', () => ({ openDeleteDialog: vi.fn() }));
vi.mock('../../frontend/js/doc-editor/highlights.js', () => ({
  applyCachedHighlights: vi.fn(),
  initDocHighlightOverlay: vi.fn(),
  cleanupDocHighlightOverlay: vi.fn(),
}));
vi.mock('../../frontend/js/corpus/corpus-viewer.js', () => ({
  openKbDoc: vi.fn(),
  saveKbDoc: vi.fn(),
}));
vi.mock('../../frontend/js/corpus/corpus-knowledge-search.js', () => ({
  mountKnowledgeSearch: vi.fn(),
  triggerKnowledgeSearch: vi.fn(),
}));
vi.mock('../../frontend/js/host/constants.js', () => ({
  getGithubUserUrl: vi.fn(() => ''),
  workbenchGithubBlobBase: vi.fn(() => null),
}));
vi.mock('../../frontend/js/shared/mermaid-render.js', () => ({
  initMermaid: vi.fn(),
  renderMermaidBlocks: vi.fn().mockResolvedValue(undefined),
}));
vi.mock('../../frontend/js/router/index.js', async () => {
  const actual = await vi.importActual('../../frontend/js/router/index.js');
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
} from '../../frontend/js/notes/viewer.js';
import * as api from '../../frontend/js/host/api.js';
import { state } from '../../frontend/js/host/state.js';

function resetDom() {
  for (const id of [
    'note-outlet', 'md-modal', 'md-body', 'md-edit-area', 'md-panel-title', 'md-close', 'md-backdrop',
    'date-heading', 'doc-list',
    'md-commit-dialog', 'comment-dialog', 'btn-edit', 'btn-add-comment', 'btn-save',
    'btn-cancel-edit', 'btn-panel-commit', 'md-github-link', 'md-lang-bar', 'md-file-size',
    'md-links-bar', 'md-tags-bar', 'knowledge-panel', 'md-commit-bar', 'md-commit-msg',
    'md-commit-result', 'btn-commit-file', 'md-btn-commit-cancel', 'md-btn-commit-ok',
    'md-btn-revert-all', 'md-commit-dialog-msg', 'md-commit-dialog-result',
    'md-commit-file-list', 'btn-copy-http', 'btn-copy-path', 'btn-goto-kb', 'btn-open-iterm',
    'comment-float-nav',
  ]) {
    const el = makeEl(id);
    el.style.display = id === 'md-edit-area' || id === 'btn-save' || id === 'btn-cancel-edit'
      || id === 'btn-panel-commit' || id === 'md-commit-bar' || id === 'md-lang-bar'
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
  locationStub.hash = '#/workbench?date=20260710';
  state.ui.activeDate = '20260710';
  state.viewer.entry = null;
  state.viewer.rawText = '';
  state.viewer.createSession = null;
  state.index.diffStatus = new Map();
  vi.clearAllMocks();
  api.archiveDocument.mockResolvedValue({
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
    expect(locationStub.hash).toBe('#/workbench?date=20260710');
  });

  it('create success: navigate note=common_path from archiveDocument', async () => {
    await openCreateNote({ temp_id: 'tmp-ok' });
    makeEl('md-edit-area').value = 'hello archived note';
    vi.clearAllMocks();
    api.archiveDocument.mockResolvedValue({
      ok: true,
      id: 'b'.repeat(32),
      common_path: 'inbox/notes/202607101431-body.md',
    });
    api.clearNoteDraft.mockResolvedValue({ ok: true });
    navigateToNoteMock.mockReturnValue(true);

    await closeModal();

    expect(api.archiveDocument).toHaveBeenCalledWith({
      body: 'hello archived note',
      source_type: 'note',
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
    api.archiveDocument.mockRejectedValueOnce(new Error('409: conflict'));

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
