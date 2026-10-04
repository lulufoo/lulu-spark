import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const { makeEl, clearDom } = vi.hoisted(() => {
  const elements = {};
  const clearDom = () => {
    for (const key of Object.keys(elements)) delete elements[key];
  };
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
      },
      innerHTML: '',
      textContent: '',
      value: '',
      disabled: false,
    };
    if (id) elements[id] = el;
    return el;
  };
  globalThis.document = {
    getElementById: (id) => makeEl(id),
    addEventListener: () => {},
  };
  return { makeEl, clearDom };
});

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchDiffStatus: vi.fn().mockResolvedValue({
    new: [], modified: ['raw/a.md'], deleted: [], renamed: [], conflicted: [],
    total: 1, ahead: 0,
  }),
  commitFiles: vi.fn(),
}));

vi.mock('../../frontend/src/toast.tsx', () => ({
  showToast: vi.fn(),
}));

import * as api from '../../frontend/src/host/api.ts';
import { showToast } from '../../frontend/src/toast.tsx';

function seedDom() {
  makeEl('btn-push-index');
  makeEl('spark-commit-dialog');
  makeEl('spark-commit-file-list');
  makeEl('spark-commit-result');
  makeEl('spark-commit-msg');
  makeEl('btn-spark-commit-ok');
  makeEl('btn-spark-commit-cancel');
}

describe('spark commit dialog — delayed close + background submit', () => {
  let resolveCommit;
  let rejectCommit;
  let doSparkCommit;
  let closeSparkCommitDialog;

  beforeEach(async () => {
    vi.useFakeTimers();
    vi.resetModules();
    vi.clearAllMocks();
    clearDom();
    seedDom();
    resolveCommit = undefined;
    rejectCommit = undefined;
    api.commitFiles.mockImplementation(
      () => new Promise((resolve, reject) => {
        resolveCommit = resolve;
        rejectCommit = reject;
      }),
    );
    const mod = await import('../../frontend/src/app-shell/commands/spark-commit-dialog.ts');
    doSparkCommit = mod.doSparkCommit;
    closeSparkCommitDialog = mod.closeSparkCommitDialog;
    makeEl('spark-commit-dialog').classList.add('open');
    makeEl('btn-push-index').disabled = false;
    makeEl('btn-push-index').textContent = '↑ Commit changes';
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('AC1: keeps open class within 500ms of clicking submit', async () => {
    doSparkCommit();
    await Promise.resolve();

    const dialog = makeEl('spark-commit-dialog');
    expect(dialog.classList.contains('open')).toBe(true);
    expect(api.commitFiles).not.toHaveBeenCalled();
  });

  it('AC2: removes open class and calls commitFiles after 500ms', async () => {
    doSparkCommit();
    await Promise.resolve();

    vi.advanceTimersByTime(500);
    await Promise.resolve();

    const dialog = makeEl('spark-commit-dialog');
    expect(dialog.classList.contains('open')).toBe(false);
    expect(api.commitFiles).toHaveBeenCalledTimes(1);
    expect(resolveCommit).toBeTypeOf('function');
  });

  it('AC3: keeps btn-push-index enabled while commit is in flight', async () => {
    doSparkCommit();
    vi.advanceTimersByTime(500);
    await Promise.resolve();

    expect(makeEl('btn-push-index').disabled).toBe(false);
  });

  it('AC4: shows success toast after api.commitFiles resolves', async () => {
    doSparkCommit();
    vi.advanceTimersByTime(500);
    await Promise.resolve();

    resolveCommit({ ok: true });
    await Promise.resolve();

    expect(showToast).toHaveBeenCalledWith('✓ Committed and pushed', 'success');
  });

  it('AC4: shows error toast with Commit failed prefix on reject', async () => {
    doSparkCommit();
    vi.advanceTimersByTime(500);
    await Promise.resolve();

    rejectCommit(new Error('push failed'));
    await Promise.resolve();
    await Promise.resolve();

    expect(showToast).toHaveBeenCalledWith('Commit failed: push failed', 'error');
  });

  it('AC5: backdrop click during delay closes dialog without committing', async () => {
    doSparkCommit();
    await Promise.resolve();

    closeSparkCommitDialog();

    expect(makeEl('spark-commit-dialog').classList.contains('open')).toBe(false);
    vi.advanceTimersByTime(500);
    await Promise.resolve();
    expect(api.commitFiles).not.toHaveBeenCalled();
  });

  it('AC6: cancel during delay does not call commitFiles', async () => {
    doSparkCommit();
    await Promise.resolve();

    closeSparkCommitDialog();

    vi.advanceTimersByTime(500);
    await Promise.resolve();
    expect(api.commitFiles).not.toHaveBeenCalled();
  });

  it('A2: double-click submit schedules only one commitFiles call', async () => {
    doSparkCommit();
    doSparkCommit();
    vi.advanceTimersByTime(500);
    await Promise.resolve();

    expect(api.commitFiles).toHaveBeenCalledTimes(1);
  });
});
