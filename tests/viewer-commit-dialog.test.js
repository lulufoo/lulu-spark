// Node environment — uses global document stub with event capturing
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

// ── Global document stub with event listener capture ──────────────────────
// vi.hoisted: runs before ESM imports, sets up globalThis.document

const { elements, makeEl, trigger } = vi.hoisted(() => {
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
      removeAttribute() {},
      remove() { if (id && elements[id] === this) delete elements[id]; },
      closest: () => null,
    };
    if (id) elements[id] = el;
    return el;
  };

  // Simulates a DOM event by calling all registered handlers for the event.
  const trigger = async (id, event, eventData = {}) => {
    const el = id ? (elements[id] || makeEl(id)) : null;
    const handlers = el?._listeners[event] || [];
    for (const fn of handlers) {
      await fn(eventData);
    }
  };

  globalThis.document = {
    getElementById: (id) => (id ? makeEl(id) : null),
    addEventListener: () => {},
    createElement: () => makeEl(),
    createTextNode: (text) => ({ textContent: text }),
    querySelectorAll: () => [],
    querySelector: () => null,
    body: { style: {}, appendChild: () => {} },
    dispatchEvent: () => {},
  };
  globalThis.requestAnimationFrame = (fn) => fn();
  globalThis.marked = undefined;

  return { elements, makeEl, trigger };
});

// ── Mock all viewer.js dependencies ──────────────────────────────────────

const mockDiffData = () => ({
  new: [], modified: [], deleted: [], renamed: [], conflicted: [], total: 0, ahead: 0,
});

vi.mock('../frontend/js/host/api.js', () => ({
  fetchFileContent: vi.fn().mockResolvedValue('# Test'),
  fetchAnnotation: vi.fn().mockResolvedValue({}),
  fetchDiffStatus: vi.fn().mockResolvedValue({ new: [], modified: [], deleted: [], renamed: [], conflicted: [], total: 0, ahead: 0 }),
  saveFile: vi.fn(),
  commitFiles: vi.fn().mockResolvedValue({ ok: true }),
  revertFile: vi.fn().mockResolvedValue({ ok: true }),
}));
vi.mock('../frontend/js/host/state.js', async () => {
  const actual = await vi.importActual('../frontend/js/host/state.js');
  return { ...actual, loadDiffStatus: vi.fn().mockResolvedValue(undefined) };
});
vi.mock('../frontend/js/notes/cards.js', () => ({
  updateTitlesInDOM: vi.fn(),
  updateDiffInDOM: vi.fn(),
}));
vi.mock('../frontend/js/notes/links-bar.js', () => ({ renderLinksBar: vi.fn() }));
vi.mock('../frontend/js/notes/tags-bar.js', () => ({ renderTagsBar: vi.fn() }));
vi.mock('../frontend/js/notes/comments.js', () => ({ renderComments: vi.fn() }));
vi.mock('../frontend/js/components/modals/delete-dialog.js', () => ({ openDeleteDialog: vi.fn() }));
vi.mock('../frontend/js/notes/highlights.js', () => ({
  applyHighlights: vi.fn(),
  initHighlightUI: vi.fn(),
}));
vi.mock('../frontend/js/corpus/kb-viewer.js', () => ({
  openKbDoc: vi.fn(),
  saveKbDoc: vi.fn(),
}));
vi.mock('../frontend/js/corpus/knowledge-search.js', () => ({
  mountKnowledgeSearch: vi.fn(),
  triggerKnowledgeSearch: vi.fn(),
}));
vi.mock('../frontend/js/host/constants.js', () => ({
  getGithubUserUrl: vi.fn(() => ''),
  workbenchGithubBlobBase: vi.fn(() => null),
}));

import { openCommitDialog } from '../frontend/js/notes/viewer.js';
import * as api from '../frontend/js/host/api.js';

// ── openCommitDialog ──────────────────────────────────────────────────────

describe('openCommitDialog', () => {
  beforeEach(() => {
    makeEl('md-commit-dialog').classList.remove('open');
    makeEl('md-commit-file-list').innerHTML = '';
    makeEl('md-commit-dialog-msg').value = 'leftover';
    makeEl('md-commit-dialog-result').textContent = 'leftover';
    makeEl('md-btn-commit-ok').disabled = false;
    api.fetchDiffStatus.mockResolvedValue(mockDiffData());
    vi.clearAllMocks();
    api.fetchDiffStatus.mockResolvedValue(mockDiffData());
  });

  it('opens the dialog', async () => {
    await openCommitDialog();
    expect(makeEl('md-commit-dialog').classList.contains('open')).toBe(true);
  });

  it('calls api.fetchDiffStatus', async () => {
    await openCommitDialog();
    expect(api.fetchDiffStatus).toHaveBeenCalled();
  });

  it('clears message input and result span', async () => {
    await openCommitDialog();
    expect(makeEl('md-commit-dialog-msg').value).toBe('');
    expect(makeEl('md-commit-dialog-result').textContent).toBe('');
  });

  it('disables OK button when total=0 and ahead=0', async () => {
    await openCommitDialog();
    expect(makeEl('md-btn-commit-ok').disabled).toBe(true);
  });

  it('populates file list when there are modified files', async () => {
    api.fetchDiffStatus.mockResolvedValueOnce({
      ...mockDiffData(), modified: ['raw/ai/note.md'], total: 1,
    });
    await openCommitDialog();
    expect(makeEl('md-commit-file-list').innerHTML).toContain('raw/ai/note.md');
  });

  it('includes revert buttons for modified files', async () => {
    api.fetchDiffStatus.mockResolvedValueOnce({
      ...mockDiffData(), modified: ['raw/ai/note.md'], total: 1,
    });
    await openCommitDialog();
    expect(makeEl('md-commit-file-list').innerHTML).toContain('Revert');
  });

  it('shows error in file list when fetchDiffStatus rejects', async () => {
    api.fetchDiffStatus.mockRejectedValueOnce(new Error('network error'));
    await openCommitDialog();
    expect(makeEl('md-commit-file-list').innerHTML).toContain('Failed to get status');
  });
});

// ── Cancel button ─────────────────────────────────────────────────────────

describe('cancel button', () => {
  it('click closes the dialog', async () => {
    makeEl('md-commit-dialog').classList.add('open');
    await trigger('md-btn-commit-cancel', 'click');
    expect(makeEl('md-commit-dialog').classList.contains('open')).toBe(false);
  });
});

// ── OK button ─────────────────────────────────────────────────────────────

describe('OK button', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.commitFiles.mockResolvedValue({ ok: true });
    api.fetchDiffStatus.mockResolvedValue({ total: 0, ahead: 0, modified: [], new: [] });
    makeEl('md-commit-dialog').classList.add('open');
    makeEl('md-commit-dialog-msg').value = '';
    makeEl('md-commit-dialog-result').textContent = '';
    makeEl('md-btn-commit-ok').disabled = false;
  });

  it('calls api.commitFiles with input message', async () => {
    makeEl('md-commit-dialog-msg').value = 'my commit message';
    await trigger('md-btn-commit-ok', 'click');
    expect(api.commitFiles).toHaveBeenCalledWith('my commit message');
  });

  it('calls api.commitFiles with default message when input is empty', async () => {
    makeEl('md-commit-dialog-msg').value = '';
    await trigger('md-btn-commit-ok', 'click');
    expect(api.commitFiles).toHaveBeenCalledWith('update: edit via viewer');
  });

  it('shows error and re-enables button on failure', async () => {
    api.commitFiles.mockRejectedValueOnce(new Error('push failed'));
    await trigger('md-btn-commit-ok', 'click');
    expect(makeEl('md-commit-dialog-result').textContent).toContain('push failed');
    expect(makeEl('md-btn-commit-ok').disabled).toBe(false);
  });
});

// ── Revert-all button ─────────────────────────────────────────────────────

describe('revert-all button', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    const btn = makeEl('md-btn-revert-all');
    btn.classList.remove('confirm');
    btn.textContent = 'Revert all changes';
    btn.disabled = false;
    api.revertFile.mockResolvedValue({ ok: true });
    api.fetchDiffStatus.mockResolvedValue({ total: 0, ahead: 0, modified: [], new: [] });
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('first click adds confirm class', async () => {
    await trigger('md-btn-revert-all', 'click');
    expect(makeEl('md-btn-revert-all').classList.contains('confirm')).toBe(true);
  });

  it('confirm state auto-resets after timeout', async () => {
    await trigger('md-btn-revert-all', 'click');
    vi.runAllTimers();
    expect(makeEl('md-btn-revert-all').classList.contains('confirm')).toBe(false);
  });

  it('second click calls api.revertFile with empty path and type', async () => {
    makeEl('md-btn-revert-all').classList.add('confirm');
    await trigger('md-btn-revert-all', 'click');
    expect(api.revertFile).toHaveBeenCalledWith('', '');
  });
});
