// Node environment — document stub + mocked viewer deps (same pattern as viewer-commit-dialog)
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const { elements, makeEl, locationStub, navigateToNoteMock, navigateBackToListMock } = vi.hoisted(() => {
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
  const navigateBackToListMock = vi.fn();

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

  return { elements, makeEl, locationStub, navigateToNoteMock, navigateBackToListMock };
});

vi.mock('../../frontend/js/host/api.js', () => ({
  fetchFileContent: vi.fn().mockResolvedValue('# Test'),
  fetchAnnotation: vi.fn().mockResolvedValue({}),
  fetchDiffStatus: vi.fn().mockResolvedValue({
    new: [], modified: [], deleted: [], renamed: [], conflicted: [], total: 0, ahead: 0,
  }),
  saveFile: vi.fn(),
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
    navigateBackToList: (...args) => navigateBackToListMock(...args),
  };
});

import { openCreateNote, closeModal, openDoc } from '../../frontend/js/notes/viewer.js';
import * as api from '../../frontend/js/host/api.js';
import { state } from '../../frontend/js/host/state.js';

const CREATE_CHROME_HIDDEN_IDS = [
  'btn-edit',
  'btn-add-comment',
  'btn-save',
  'btn-cancel-edit',
  'btn-panel-commit',
  'md-github-link',
  'md-lang-bar',
  'md-file-size',
  'md-links-bar',
  'md-tags-bar',
  'knowledge-panel',
  'btn-copy-http',
  'btn-copy-path',
  'btn-goto-kb',
  'btn-open-iterm',
  'comment-float-nav',
  'md-commit-bar',
];

function resetViewerDom() {
  for (const id of [
    'note-outlet', 'md-modal', 'md-body', 'md-edit-area', 'md-panel-title', 'md-close', 'md-backdrop',
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
  vi.clearAllMocks();
  api.archiveDocument.mockResolvedValue({
    ok: true,
    id: 'a'.repeat(32),
    common_path: 'inbox/notes/202607101430-hello.md',
  });
  api.saveNoteDraft.mockResolvedValue({ ok: true });
  api.clearNoteDraft.mockResolvedValue({ ok: true });
  api.getNoteDraft.mockResolvedValue({ content: '' });
  navigateToNoteMock.mockReturnValue(true);
  navigateBackToListMock.mockClear();
  globalThis.alert.mockClear();
  document.dispatchEvent.mockClear();
}

describe('openCreateNote', () => {
  beforeEach(() => {
    resetViewerDom();
  });

  it('enters create session bound to drafts/notes/<temp_id>', async () => {
    await openCreateNote({ temp_id: 'tmp-abc' });

    expect(makeEl('note-outlet').hidden).toBe(false);
    expect(makeEl('md-modal').style.display).toBe('none');
    expect(makeEl('md-edit-area').style.display).not.toBe('none');
    expect(api.saveNoteDraft).toHaveBeenCalledWith('tmp-abc', expect.any(String));
    expect(state.viewer.createSession?.tempId).toBe('tmp-abc');
    expect(state.viewer.createSession?.status).toBe('creating');
    expect(state.viewer.entry).toBeNull();
  });

  it('applies create chrome: body-only, no shell-field controls', async () => {
    await openCreateNote({ temp_id: 'tmp-chrome' });

    for (const id of CREATE_CHROME_HIDDEN_IDS) {
      expect(makeEl(id).style.display, id).toBe('none');
    }
    // No source_type / topic / H1 / 创建时间 controls in create chrome
    expect(document.querySelector('#create-source-type')).toBeNull();
    expect(document.querySelector('#create-topic')).toBeNull();
    expect(document.querySelector('#create-h1')).toBeNull();
    expect(document.querySelector('#create-created-at')).toBeNull();
    expect(makeEl('note-outlet').classList.contains('is-create')).toBe(true);
    // Close stays available (not in hide list)
    expect(makeEl('md-close').style.display).not.toBe('none');
  });

  it('restores persisted chrome display when create session is cleared', async () => {
    makeEl('btn-copy-http').style.display = '';
    makeEl('btn-copy-path').style.display = '';
    await openCreateNote({ temp_id: 'tmp-restore' });
    expect(makeEl('btn-copy-http').style.display).toBe('none');
    await closeModal();
    expect(makeEl('note-outlet').classList.contains('is-create')).toBe(false);
    expect(makeEl('btn-copy-http').style.display).toBe('');
    expect(makeEl('btn-copy-path').style.display).toBe('');
  });

  it('reuses the same viewer chrome tree (no second editor route)', async () => {
    await openCreateNote({ temp_id: 'tmp-same' });
    expect(makeEl('note-outlet').hidden).toBe(false);
    expect(makeEl('md-edit-area').style.display).not.toBe('none');
    // Same exports as edit path — openDoc still available; no alternate editor module
    expect(typeof openDoc).toBe('function');
    expect(typeof openCreateNote).toBe('function');
  });

  it('does not enter create when openCreateNote fails; keeps prior UI', async () => {
    makeEl('note-outlet').hidden = true;
    makeEl('md-modal').style.display = 'none';
    api.saveNoteDraft.mockRejectedValueOnce(new Error('draft write failed'));

    await openCreateNote({ temp_id: 'tmp-fail' });

    expect(makeEl('note-outlet').hidden).toBe(true);
    expect(makeEl('md-modal').style.display).toBe('none');
    expect(state.viewer.createSession).toBeNull();
    expect(makeEl('note-outlet').classList.contains('is-create')).toBe(false);
    expect(globalThis.alert).toHaveBeenCalledWith(
      expect.stringContaining('Could not open new note'),
    );
  });
});

describe('create session exit', () => {
  beforeEach(async () => {
    resetViewerDom();
    await openCreateNote({ temp_id: 'tmp-exit' });
    vi.clearAllMocks();
    api.archiveDocument.mockResolvedValue({
      ok: true,
      id: 'b'.repeat(32),
      common_path: 'inbox/notes/202607101431-body.md',
    });
    api.clearNoteDraft.mockResolvedValue({ ok: true });
    navigateToNoteMock.mockReturnValue(true);
    navigateBackToListMock.mockClear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('trim-nonempty exit: Primary create via archiveDocument, then navigate note=common_path', async () => {
    makeEl('md-edit-area').value = '  hello note\n';
    locationStub.hash = '#/workbench?date=20260710';

    await closeModal();

    expect(api.archiveDocument).toHaveBeenCalledWith({
      body: 'hello note',
      source_type: 'note',
    });
    expect(api.clearNoteDraft).toHaveBeenCalledWith('tmp-exit');
    expect(navigateToNoteMock).toHaveBeenCalledWith({
      date: '20260710',
      note: 'inbox/notes/202607101431-body.md',
    });
    // Pass via navigate + outlet state — not modal.display alone (chap-vf)
    expect(makeEl('note-outlet').hidden).toBe(false);
    expect(navigateBackToListMock).not.toHaveBeenCalled();
    expect(state.viewer.createSession).toBeNull();
  });

  it('empty exit: clear draft + closeModal, no Primary create', async () => {
    makeEl('md-edit-area').value = '   \n  ';

    await closeModal();

    expect(api.archiveDocument).not.toHaveBeenCalled();
    expect(api.clearNoteDraft).toHaveBeenCalledWith('tmp-exit');
    expect(makeEl('note-outlet').hidden).toBe(true);
    expect(state.viewer.createSession).toBeNull();
    expect(navigateToNoteMock).not.toHaveBeenCalled();
    expect(navigateBackToListMock).toHaveBeenCalledWith({ date: '20260710' });
  });

  it('archive failure: alert, keep create session + draft, no Annotation API', async () => {
    makeEl('md-edit-area').value = 'retry me';
    api.archiveDocument.mockRejectedValueOnce(new Error('409: conflict'));

    await closeModal();

    expect(globalThis.alert).toHaveBeenCalledWith(expect.stringContaining('409: conflict'));
    expect(api.clearNoteDraft).not.toHaveBeenCalled();
    expect(makeEl('note-outlet').hidden).toBe(false);
    expect(makeEl('note-outlet').dataset.wbMode).toBe('create');
    expect(state.viewer.createSession?.tempId).toBe('tmp-exit');
    expect(state.viewer.createSession?.status).toBe('creating');
    expect(api.archiveDocument).toHaveBeenCalled();
    expect(navigateToNoteMock).not.toHaveBeenCalled();
    expect(navigateBackToListMock).not.toHaveBeenCalled();
  });

  it('empty exit lands list via navigateBackToList (history.back or list location)', async () => {
    makeEl('md-edit-area').value = '';
    locationStub.hash = '#/workbench?date=20260710';

    await closeModal();

    expect(api.archiveDocument).not.toHaveBeenCalled();
    expect(api.clearNoteDraft).toHaveBeenCalledWith('tmp-exit');
    expect(state.viewer.createSession).toBeNull();
    expect(makeEl('note-outlet').hidden).toBe(true);
    expect(navigateToNoteMock).not.toHaveBeenCalled();
    // Ban-hash retired: close/exit must observe navigateBackToList (list location / history.back)
    expect(navigateBackToListMock).toHaveBeenCalledWith({ date: '20260710' });
  });

  it('create flow has no source_type/topic mutation controls', async () => {
    expect(document.querySelector('[name="source_type"]')).toBeNull();
    expect(document.querySelector('[name="topic"]')).toBeNull();
    expect(document.querySelector('#create-source-type')).toBeNull();
    expect(document.querySelector('#create-topic')).toBeNull();
  });
});
