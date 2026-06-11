import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const { makeEl, trigger, clearDom } = vi.hoisted(() => {
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
      _listeners: {},
      addEventListener(event, fn) {
        if (!this._listeners[event]) this._listeners[event] = [];
        this._listeners[event].push(fn);
      },
      innerHTML: '',
      textContent: '',
      value: '',
      disabled: false,
    };
    if (id) elements[id] = el;
    return el;
  };
  const trigger = async (id, event, eventData = {}) => {
    const el = makeEl(id);
    for (const fn of el._listeners[event] || []) await fn(eventData);
  };
  globalThis.document = {
    getElementById: (id) => makeEl(id),
    addEventListener: () => {},
  };
  return { makeEl, trigger, clearDom };
});

vi.mock('../frontend/js/api.js', () => ({
  fetchDiffStatus: vi.fn().mockResolvedValue({
    new: [], modified: ['raw/a.md'], deleted: [], renamed: [], conflicted: [],
    total: 1, ahead: 0,
  }),
  commitFiles: vi.fn(),
}));

vi.mock('../frontend/js/components/toast.js', () => ({
  showToast: vi.fn(),
}));

import * as api from '../frontend/js/api.js';
import { showToast } from '../frontend/js/components/toast.js';

function seedDom() {
  makeEl('btn-push-index');
  makeEl('commit-changes-dialog');
  makeEl('commit-changes-file-list');
  makeEl('commit-changes-result');
  makeEl('commit-changes-msg');
  makeEl('btn-commit-changes-ok');
  makeEl('btn-commit-changes-cancel');
}

describe('homepage commit dialog — delayed close + background submit', () => {
  let resolveCommit;
  let rejectCommit;

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
    await import('../frontend/js/components/modals/commit-dialog.js');
    makeEl('commit-changes-dialog').classList.add('open');
    makeEl('btn-push-index').disabled = false;
    makeEl('btn-push-index').textContent = '↑ 提交变更';
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('AC1: keeps open class within 500ms of clicking submit', async () => {
    void trigger('btn-commit-changes-ok', 'click');
    await Promise.resolve();

    const dialog = makeEl('commit-changes-dialog');
    expect(dialog.classList.contains('open')).toBe(true);
    expect(api.commitFiles).not.toHaveBeenCalled();
  });

  it('AC2: removes open class and calls commitFiles after 500ms', async () => {
    void trigger('btn-commit-changes-ok', 'click');
    await Promise.resolve();

    vi.advanceTimersByTime(500);
    await Promise.resolve();

    const dialog = makeEl('commit-changes-dialog');
    expect(dialog.classList.contains('open')).toBe(false);
    expect(api.commitFiles).toHaveBeenCalledTimes(1);
    expect(resolveCommit).toBeTypeOf('function');
  });

  it('AC3: keeps btn-push-index enabled while commit is in flight', async () => {
    void trigger('btn-commit-changes-ok', 'click');
    vi.advanceTimersByTime(500);
    await Promise.resolve();

    expect(makeEl('btn-push-index').disabled).toBe(false);
  });

  it('AC4: shows success toast after api.commitFiles resolves', async () => {
    void trigger('btn-commit-changes-ok', 'click');
    vi.advanceTimersByTime(500);
    await Promise.resolve();

    resolveCommit({ ok: true });
    await Promise.resolve();

    expect(showToast).toHaveBeenCalledWith('✓ 提交并推送成功', 'success');
  });

  it('AC4: shows error toast with 提交失败 prefix on reject', async () => {
    void trigger('btn-commit-changes-ok', 'click');
    vi.advanceTimersByTime(500);
    await Promise.resolve();

    rejectCommit(new Error('push failed'));
    await Promise.resolve();
    await Promise.resolve();

    expect(showToast).toHaveBeenCalledWith('提交失败：push failed', 'error');
  });

  it('AC5: backdrop click during delay closes dialog without committing', async () => {
    void trigger('btn-commit-changes-ok', 'click');
    await Promise.resolve();

    const dialog = makeEl('commit-changes-dialog');
    const backdropHandler = dialog._listeners.click?.[0];
    backdropHandler?.({ target: dialog });

    expect(dialog.classList.contains('open')).toBe(false);
    vi.advanceTimersByTime(500);
    await Promise.resolve();
    expect(api.commitFiles).not.toHaveBeenCalled();
  });

  it('AC6: cancel during delay does not call commitFiles', async () => {
    void trigger('btn-commit-changes-ok', 'click');
    await Promise.resolve();

    await trigger('btn-commit-changes-cancel', 'click');

    vi.advanceTimersByTime(500);
    await Promise.resolve();
    expect(api.commitFiles).not.toHaveBeenCalled();
  });

  it('A2: double-click submit schedules only one commitFiles call', async () => {
    void trigger('btn-commit-changes-ok', 'click');
    void trigger('btn-commit-changes-ok', 'click');
    vi.advanceTimersByTime(500);
    await Promise.resolve();

    expect(api.commitFiles).toHaveBeenCalledTimes(1);
  });
});
