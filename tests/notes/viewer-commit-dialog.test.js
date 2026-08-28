// Node environment — uses global document stub with event capturing
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

function serializeTestNode(node) {
  if (node == null || node === false) return '';
  if (typeof node === 'string' || typeof node === 'number') return String(node);
  if (Array.isArray(node)) return node.map(serializeTestNode).join('');
  const type = node.type;
  const props = node.props || {};
  if (typeof type !== 'string') return serializeTestNode(props.children);
  const { children, className, style: _style, ...rest } = props;
  let attrs = className ? ` class="${className}"` : '';
  for (const [key, value] of Object.entries(rest)) {
    if (value == null || value === false || key === 'children') continue;
    if (typeof value === 'object') continue;
    attrs += ` ${key}="${value}"`;
  }
  const inner = serializeTestNode(children);
  if (type === 'img' || type === 'input' || type === 'br') {
    return `<${type}${attrs}>`;
  }
  return `<${type}${attrs}>${inner}</${type}>`;
}

vi.mock('../../frontend/src/island.ts', () => ({
  renderToHtml: (node) => serializeTestNode(node),
}));


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
vi.mock('../../frontend/src/notes/cards.tsx', () => ({
  updateTitlesInDOM: vi.fn(),
  updateDiffInDOM: vi.fn(),
}));

import { closeCommitDialog, doMdCommit, doMdRevertAll, openCommitDialog } from '../../frontend/src/notes/viewer/commit.tsx';
import * as api from '../../frontend/src/host/api.ts';

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

// ── Cancel ────────────────────────────────────────────────────────────────

describe('cancel', () => {
  it('closeCommitDialog removes the open class', () => {
    makeEl('md-commit-dialog').classList.add('open');
    closeCommitDialog();
    expect(makeEl('md-commit-dialog').classList.contains('open')).toBe(false);
  });
});

// ── OK ────────────────────────────────────────────────────────────────────

describe('OK', () => {
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
    await doMdCommit();
    expect(api.commitFiles).toHaveBeenCalledWith('my commit message');
  });

  it('calls api.commitFiles with default message when input is empty', async () => {
    makeEl('md-commit-dialog-msg').value = '';
    await doMdCommit();
    expect(api.commitFiles).toHaveBeenCalledWith('update: edit via viewer');
  });

  it('shows error and re-enables button on failure', async () => {
    api.commitFiles.mockRejectedValueOnce(new Error('push failed'));
    await doMdCommit();
    expect(makeEl('md-commit-dialog-result').textContent).toContain('push failed');
    expect(makeEl('md-btn-commit-ok').disabled).toBe(false);
  });
});

// ── Revert-all ────────────────────────────────────────────────────────────

describe('revert-all', () => {
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
    await doMdRevertAll();
    expect(makeEl('md-btn-revert-all').classList.contains('confirm')).toBe(true);
  });

  it('confirm state auto-resets after timeout', async () => {
    await doMdRevertAll();
    vi.runAllTimers();
    expect(makeEl('md-btn-revert-all').classList.contains('confirm')).toBe(false);
  });

  it('second click calls api.revertFile with empty path and type', async () => {
    makeEl('md-btn-revert-all').classList.add('confirm');
    await doMdRevertAll();
    expect(api.revertFile).toHaveBeenCalledWith('', '');
  });
});
