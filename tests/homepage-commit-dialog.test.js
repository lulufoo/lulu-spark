import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const { makeEl, trigger } = vi.hoisted(() => {
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
  return { makeEl, trigger };
});

vi.mock('../frontend/js/api.js', () => ({
  fetchDiffStatus: vi.fn().mockResolvedValue({
    new: [], modified: ['raw/a.md'], deleted: [], renamed: [], conflicted: [],
    total: 1, ahead: 0,
  }),
  commitFiles: vi.fn(),
}));

import * as api from '../frontend/js/api.js';

function seedDom() {
  makeEl('btn-push-index');
  makeEl('commit-changes-dialog');
  makeEl('commit-changes-file-list');
  makeEl('commit-changes-result');
  makeEl('commit-changes-msg');
  makeEl('btn-commit-changes-ok');
  makeEl('btn-commit-changes-cancel');
}

describe('homepage commit dialog — busy state', () => {
  beforeEach(async () => {
    vi.resetModules();
    vi.clearAllMocks();
    vi.useFakeTimers();
    seedDom();
    api.commitFiles.mockImplementation(() => new Promise(() => {}));
    await import('../frontend/js/components/modals/commit-dialog.js');
    makeEl('commit-changes-dialog').classList.add('open');
    makeEl('btn-commit-changes-ok').disabled = false;
    makeEl('btn-commit-changes-cancel').disabled = false;
    makeEl('commit-changes-msg').disabled = false;
    makeEl('btn-push-index').disabled = false;
    makeEl('btn-push-index').textContent = '↑ 提交变更';
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('locks header and dialog controls while commit is in flight', async () => {
    void trigger('btn-commit-changes-ok', 'click');
    await Promise.resolve();
    expect(makeEl('btn-push-index').disabled).toBe(true);
    expect(makeEl('btn-push-index').textContent).toBe('提交中…');
    expect(makeEl('btn-commit-changes-ok').disabled).toBe(true);
    expect(makeEl('btn-commit-changes-cancel').disabled).toBe(true);
    expect(makeEl('commit-changes-msg').disabled).toBe(true);
    expect(makeEl('commit-changes-result').textContent).toBe('提交中…');
  });

  it('does not close dialog on backdrop click while committing', async () => {
    void trigger('btn-commit-changes-ok', 'click');
    await Promise.resolve();
    const dialog = makeEl('commit-changes-dialog');
    const backdropHandler = dialog._listeners.click?.[0];
    backdropHandler?.({ target: dialog });
    expect(dialog.classList.contains('open')).toBe(true);
  });
});

describe('homepage commit dialog — success close', () => {
  beforeEach(async () => {
    vi.resetModules();
    vi.clearAllMocks();
    vi.useFakeTimers();
    seedDom();
    api.commitFiles.mockResolvedValue({ ok: true });
    await import('../frontend/js/components/modals/commit-dialog.js');
    makeEl('commit-changes-dialog').classList.add('open');
  });

  afterEach(() => vi.useRealTimers());

  it('closes dialog 500ms after success and restores header', async () => {
    const click = trigger('btn-commit-changes-ok', 'click');
    await Promise.resolve();
    await vi.runAllTimersAsync();
    await click;
    expect(makeEl('commit-changes-dialog').classList.contains('open')).toBe(false);
    expect(makeEl('btn-push-index').disabled).toBe(false);
    expect(makeEl('btn-push-index').textContent).toBe('↑ 提交变更');
  });
});

describe('homepage commit dialog — error unlock', () => {
  beforeEach(async () => {
    vi.resetModules();
    vi.clearAllMocks();
    seedDom();
    api.commitFiles.mockRejectedValue(new Error('push failed'));
    await import('../frontend/js/components/modals/commit-dialog.js');
    makeEl('commit-changes-dialog').classList.add('open');
  });

  it('unlocks controls and keeps dialog open on failure', async () => {
    await trigger('btn-commit-changes-ok', 'click');
    expect(makeEl('commit-changes-dialog').classList.contains('open')).toBe(true);
    expect(makeEl('commit-changes-result').textContent).toContain('push failed');
    expect(makeEl('btn-push-index').disabled).toBe(false);
    expect(makeEl('btn-commit-changes-ok').disabled).toBe(false);
    expect(makeEl('btn-commit-changes-cancel').disabled).toBe(false);
    expect(makeEl('commit-changes-msg').disabled).toBe(false);
  });
});
