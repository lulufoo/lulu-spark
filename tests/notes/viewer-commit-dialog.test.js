// Node environment — uses global document stub with event capturing
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

// ── Global document stub ──────────────────────────────────────────────────
// vi.hoisted: runs before ESM imports, sets up globalThis.document

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

  return { makeEl };
});

const mockDiffData = () => ({
  new: [], modified: [], deleted: [], renamed: [], conflicted: [], total: 0, ahead: 0,
});

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchDiffStatus: vi.fn().mockResolvedValue({ new: [], modified: [], deleted: [], renamed: [], conflicted: [], total: 0, ahead: 0 }),
  commitFiles: vi.fn().mockResolvedValue({ ok: true }),
  revertFile: vi.fn().mockResolvedValue({ ok: true }),
}));
vi.mock('../../frontend/src/host/state.ts', async () => {
  const actual = await vi.importActual('../../frontend/src/host/state.ts');
  return { ...actual, loadDiffStatus: vi.fn().mockResolvedValue(undefined) };
});
vi.mock('../../frontend/src/notes/ui/cards.tsx', () => ({
  updateTitlesInDOM: vi.fn(),
  updateDiffInDOM: vi.fn(),
}));

import { closeCommitDialog, doMdCommit, doMdRevertAll, openCommitDialog } from '../../frontend/src/notes/ui/viewer/commit.tsx';
import { commitOpenStore } from '../../frontend/src/notes/state/dialog-open.ts';
import { commitViewStore, emptyCommitView } from '../../frontend/src/notes/state/commit.ts';
import * as api from '../../frontend/src/host/api.ts';

function commitView() {
  return commitViewStore.getSnapshot();
}

// ── openCommitDialog ──────────────────────────────────────────────────────

describe('openCommitDialog', () => {
  beforeEach(() => {
    commitOpenStore.set(false);
    commitViewStore.set(emptyCommitView());
    makeEl('md-commit-dialog').classList.remove('open');
    api.fetchDiffStatus.mockResolvedValue(mockDiffData());
    vi.clearAllMocks();
    api.fetchDiffStatus.mockResolvedValue(mockDiffData());
  });

  it('opens the dialog', async () => {
    await openCommitDialog();
    expect(makeEl('md-commit-dialog').classList.contains('open')).toBe(true);
    expect(commitOpenStore.getSnapshot()).toBe(true);
  });

  it('calls api.fetchDiffStatus', async () => {
    await openCommitDialog();
    expect(api.fetchDiffStatus).toHaveBeenCalled();
  });

  it('clears message input and result span', async () => {
    commitViewStore.set({ ...emptyCommitView(), message: 'leftover', result: 'leftover' });
    await openCommitDialog();
    expect(commitView().message).toBe('');
    expect(commitView().result).toBe('');
  });

  it('disables OK button when total=0 and ahead=0', async () => {
    await openCommitDialog();
    expect(commitView().canCommit).toBe(false);
    expect(commitView().empty).toBe(true);
  });

  it('populates file list when there are modified files', async () => {
    api.fetchDiffStatus.mockResolvedValueOnce({
      ...mockDiffData(), modified: ['raw/ai/note.md'], total: 1,
    });
    await openCommitDialog();
    expect(commitView().groups.some((g) => g.files.includes('raw/ai/note.md'))).toBe(true);
  });

  it('includes revert buttons for modified files', async () => {
    api.fetchDiffStatus.mockResolvedValueOnce({
      ...mockDiffData(), modified: ['raw/ai/note.md'], total: 1,
    });
    await openCommitDialog();
    const modified = commitView().groups.find((g) => g.key === 'modified');
    expect(modified?.files).toContain('raw/ai/note.md');
  });

  it('shows error in file list when fetchDiffStatus rejects', async () => {
    api.fetchDiffStatus.mockRejectedValueOnce(new Error('network error'));
    await openCommitDialog();
    expect(commitView().error).toBe('network error');
  });
});

// ── Cancel ────────────────────────────────────────────────────────────────

describe('cancel', () => {
  it('closeCommitDialog removes the open class', () => {
    makeEl('md-commit-dialog').classList.add('open');
    commitOpenStore.set(true);
    closeCommitDialog();
    expect(makeEl('md-commit-dialog').classList.contains('open')).toBe(false);
    expect(commitOpenStore.getSnapshot()).toBe(false);
  });
});

// ── OK ────────────────────────────────────────────────────────────────────

describe('OK', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.commitFiles.mockResolvedValue({ ok: true });
    api.fetchDiffStatus.mockResolvedValue({ total: 0, ahead: 0, modified: [], new: [] });
    makeEl('md-commit-dialog').classList.add('open');
    commitOpenStore.set(true);
    commitViewStore.set({ ...emptyCommitView(), canCommit: true });
  });

  it('calls api.commitFiles with input message', async () => {
    commitViewStore.set({ ...emptyCommitView(), message: 'my commit message', canCommit: true });
    await doMdCommit();
    expect(api.commitFiles).toHaveBeenCalledWith('my commit message');
  });

  it('calls api.commitFiles with default message when input is empty', async () => {
    commitViewStore.set({ ...emptyCommitView(), message: '', canCommit: true });
    await doMdCommit();
    expect(api.commitFiles).toHaveBeenCalledWith('update: edit via viewer');
  });

  it('shows error and re-enables button on failure', async () => {
    api.commitFiles.mockRejectedValueOnce(new Error('push failed'));
    await doMdCommit();
    expect(commitView().result).toContain('push failed');
    expect(commitView().canCommit).toBe(true);
    expect(commitView().committing).toBe(false);
  });
});

// ── Revert-all ────────────────────────────────────────────────────────────

describe('revert-all', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    commitViewStore.set(emptyCommitView());
    api.revertFile.mockResolvedValue({ ok: true });
    api.fetchDiffStatus.mockResolvedValue({ total: 0, ahead: 0, modified: [], new: [] });
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('first click adds confirm class', async () => {
    await doMdRevertAll();
    expect(commitView().revertAllConfirm).toBe(true);
  });

  it('confirm state auto-resets after timeout', async () => {
    await doMdRevertAll();
    vi.runAllTimers();
    expect(commitView().revertAllConfirm).toBe(false);
  });

  it('second click calls api.revertFile with empty path and type', async () => {
    commitViewStore.set({ ...emptyCommitView(), revertAllConfirm: true });
    await doMdRevertAll();
    expect(api.revertFile).toHaveBeenCalledWith('', '');
  });
});
