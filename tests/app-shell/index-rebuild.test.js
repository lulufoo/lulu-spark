// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { createRoot } from 'react-dom/client';
import { act } from 'react';
import { createElement } from 'react';

const apiMocks = vi.hoisted(() => ({
  reindexAll: vi.fn(),
  getReindexAllStatus: vi.fn(),
}));

vi.mock('../../frontend/src/host/api.ts', () => ({
  reindexAll: (...args) => apiMocks.reindexAll(...args),
  getReindexAllStatus: (...args) => apiMocks.getReindexAllStatus(...args),
}));

globalThis.IS_REACT_ACT_ENVIRONMENT = true;

async function mount() {
  vi.resetModules();
  const [{ IndexRebuildButton }, cmd] = await Promise.all([
    import('../../frontend/src/app-shell/ui/index-rebuild.tsx'),
    import('../../frontend/src/app-shell/commands/index-rebuild.ts'),
  ]);
  cmd._resetIndexRebuild();
  const host = document.createElement('div');
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(createElement(IndexRebuildButton));
  });
  return { host, root, cmd };
}

function btn() {
  return document.getElementById('btn-index-rebuild');
}

describe('header index rebuild control', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    document.body.innerHTML = '';
    apiMocks.getReindexAllStatus.mockResolvedValue({ status: 'idle', log: '' });
    apiMocks.reindexAll.mockResolvedValue({ status: 'running' });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('renders one idle, clickable control with English copy', async () => {
    await mount();
    const el = btn();
    expect(el).toBeTruthy();
    expect(el.disabled).toBe(false);
    expect(el.textContent).toContain('Index');
    expect(el.title).toBe('Rebuild search index');
    expect(el.classList.contains('syncing')).toBe(false);
    expect(document.querySelectorAll('.gs-rebuild-btn')).toHaveLength(1);
  });

  it('picks up the startup rebuild: spins while running, clears on done', async () => {
    apiMocks.getReindexAllStatus
      .mockResolvedValueOnce({ status: 'running', log: 'Indexing notes…' })
      .mockResolvedValueOnce({ status: 'running', log: 'Indexing knowledge…' })
      .mockResolvedValue({ status: 'done', log: 'Notes: ok\nKnowledge: ok' });
    await mount();
    await act(async () => {});
    expect(btn().disabled).toBe(true);
    expect(btn().classList.contains('syncing')).toBe(true);
    expect(apiMocks.reindexAll).not.toHaveBeenCalled();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(2000);
    });
    expect(btn().classList.contains('syncing')).toBe(true);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(2000);
    });
    expect(btn().disabled).toBe(false);
    expect(btn().classList.contains('syncing')).toBe(false);
    expect(btn().title).toContain('last run');
  });

  it('click starts reindexAll once and polls until finished', async () => {
    await mount();
    apiMocks.getReindexAllStatus
      .mockResolvedValueOnce({ status: 'running', log: 'Indexing notes…' })
      .mockResolvedValue({ status: 'done', log: 'ok' });

    await act(async () => {
      btn().click();
    });
    expect(apiMocks.reindexAll).toHaveBeenCalledTimes(1);
    expect(btn().disabled).toBe(true);

    // Clicking while running must not start a second job.
    await act(async () => {
      btn().click();
    });
    expect(apiMocks.reindexAll).toHaveBeenCalledTimes(1);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(2000);
    });
    expect(btn().disabled).toBe(true);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(2000);
    });
    expect(btn().disabled).toBe(false);
  });

  it('surfaces backend errors in the title and re-enables the control', async () => {
    await mount();
    apiMocks.reindexAll.mockResolvedValue({ error: 'already running' });
    await act(async () => {
      btn().click();
    });
    expect(btn().disabled).toBe(false);
    expect(btn().classList.contains('error')).toBe(true);
    expect(btn().title).toContain('already running');
  });
});
