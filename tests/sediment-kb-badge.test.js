// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';

const fetchKbDocCount = vi.hoisted(() => vi.fn());

vi.mock('../frontend/js/api.js', () => ({
  fetchKbDocCount: (...args) => fetchKbDocCount(...args),
}));

import {
  BADGE_BASE_TITLE,
  formatBadgeTitle,
  getCorpusRepoFromHash,
  initSedimentKbBadge,
} from '../frontend/js/sediment-kb-badge.js';

function seedBadgeDom() {
  document.body.innerHTML = '<button id="btn-repo-menu">⚙ 沉淀知识库</button>';
}

function getBtnText() {
  return document.getElementById('btn-repo-menu')?.textContent ?? '';
}

describe('sediment-kb-badge helpers', () => {
  it('getCorpusRepoFromHash returns repo on corpus route', () => {
    expect(getCorpusRepoFromHash('#/corpus/owner/repo')).toBe('owner/repo');
  });

  it('getCorpusRepoFromHash returns null on non-corpus route', () => {
    expect(getCorpusRepoFromHash('#/home')).toBeNull();
  });

  it('getCorpusRepoFromHash returns null when corpus repo is empty', () => {
    expect(getCorpusRepoFromHash('#/corpus')).toBeNull();
  });

  it('formatBadgeTitle uses base title for null', () => {
    expect(formatBadgeTitle(null)).toBe(BADGE_BASE_TITLE);
  });

  it('formatBadgeTitle appends middle dot and integer count', () => {
    expect(formatBadgeTitle(5)).toBe(`${BADGE_BASE_TITLE} · 5`);
    expect(formatBadgeTitle(3.7)).toBe(`${BADGE_BASE_TITLE} · 3`);
  });
});

describe('initSedimentKbBadge', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useRealTimers();
    seedBadgeDom();
    window.location.hash = '#/home';
    fetchKbDocCount.mockResolvedValue(5);
  });

  it('shows base title on non-corpus route without suffix', async () => {
    initSedimentKbBadge();
    await Promise.resolve();
    expect(getBtnText()).toBe(BADGE_BASE_TITLE);
    expect(getBtnText()).not.toMatch(/·\s*\d/);
    expect(fetchKbDocCount).not.toHaveBeenCalled();
  });

  it('updates btn-repo-menu with count on corpus route after fetch', async () => {
    window.location.hash = '#/corpus/owner/repo';
    fetchKbDocCount.mockResolvedValue(5);

    initSedimentKbBadge();
    await vi.waitFor(() => {
      expect(getBtnText()).toBe(`${BADGE_BASE_TITLE} · 5`);
    });
    expect(fetchKbDocCount).toHaveBeenCalledWith('owner/repo');
  });

  it('keeps base title before fetch completes', async () => {
    window.location.hash = '#/corpus/owner/repo';
    let resolveFetch;
    fetchKbDocCount.mockImplementation(
      () => new Promise((resolve) => { resolveFetch = resolve; }),
    );

    initSedimentKbBadge();
    await Promise.resolve();
    expect(getBtnText()).toBe(BADGE_BASE_TITLE);

    resolveFetch(7);
    await vi.waitFor(() => {
      expect(getBtnText()).toBe(`${BADGE_BASE_TITLE} · 7`);
    });
  });

  it('clears suffix when leaving corpus route', async () => {
    window.location.hash = '#/corpus/owner/repo';
    initSedimentKbBadge();
    await vi.waitFor(() => {
      expect(getBtnText()).toContain('· 5');
    });

    window.location.hash = '#/home';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    expect(getBtnText()).toBe(BADGE_BASE_TITLE);
  });

  it('restores base title when fetch rejects', async () => {
    window.location.hash = '#/corpus/owner/repo';
    fetchKbDocCount.mockRejectedValue(new Error('network'));

    initSedimentKbBadge();
    await vi.waitFor(() => {
      expect(getBtnText()).toBe(BADGE_BASE_TITLE);
    });
  });

  it('discards stale count when route changes before fetch completes', async () => {
    window.location.hash = '#/corpus/owner/repoA';
    let resolveA;
    fetchKbDocCount.mockImplementation((repo) => {
      if (repo === 'owner/repoA') {
        return new Promise((resolve) => { resolveA = resolve; });
      }
      return Promise.resolve(9);
    });

    initSedimentKbBadge();
    window.location.hash = '#/corpus/owner/repoB';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    await vi.waitFor(() => {
      expect(getBtnText()).toBe(`${BADGE_BASE_TITLE} · 9`);
    });

    resolveA(3);
    await Promise.resolve();
    expect(getBtnText()).toBe(`${BADGE_BASE_TITLE} · 9`);
  });

  it('debounces kb-diff-updated before re-fetch', async () => {
    vi.useFakeTimers();
    window.location.hash = '#/corpus/owner/repo';
    fetchKbDocCount.mockResolvedValue(2);

    initSedimentKbBadge();
    await vi.waitFor(() => expect(fetchKbDocCount).toHaveBeenCalledTimes(1));

    fetchKbDocCount.mockResolvedValue(4);
    window.dispatchEvent(new CustomEvent('kb-diff-updated'));
    await vi.advanceTimersByTimeAsync(299);
    expect(fetchKbDocCount).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(1);
    await vi.waitFor(() => expect(fetchKbDocCount).toHaveBeenCalledTimes(2));
    await vi.waitFor(() => {
      expect(getBtnText()).toBe(`${BADGE_BASE_TITLE} · 4`);
    });
    vi.useRealTimers();
  });

  it('debounces kb:hide-pattern-changed before re-fetch', async () => {
    vi.useFakeTimers();
    window.location.hash = '#/corpus/owner/repo';
    fetchKbDocCount.mockResolvedValue(2);

    initSedimentKbBadge();
    await vi.waitFor(() => expect(fetchKbDocCount).toHaveBeenCalledTimes(1));

    fetchKbDocCount.mockResolvedValue(6);
    window.dispatchEvent(new CustomEvent('kb:hide-pattern-changed'));
    await vi.advanceTimersByTimeAsync(299);
    expect(fetchKbDocCount).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(1);
    await vi.waitFor(() => expect(fetchKbDocCount).toHaveBeenCalledTimes(2));
    await vi.waitFor(() => {
      expect(getBtnText()).toBe(`${BADGE_BASE_TITLE} · 6`);
    });
    vi.useRealTimers();
  });

  it('does not apply stale suffix when leaving corpus during debounce window', async () => {
    vi.useFakeTimers();
    window.location.hash = '#/corpus/owner/repo';
    fetchKbDocCount.mockResolvedValue(3);

    initSedimentKbBadge();
    await vi.waitFor(() => {
      expect(getBtnText()).toBe(`${BADGE_BASE_TITLE} · 3`);
    });

    fetchKbDocCount.mockResolvedValue(99);
    window.dispatchEvent(new CustomEvent('kb-diff-updated'));
    await vi.advanceTimersByTimeAsync(150);

    window.location.hash = '#/home';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    expect(getBtnText()).toBe(BADGE_BASE_TITLE);

    await vi.advanceTimersByTimeAsync(300);
    await Promise.resolve();
    expect(getBtnText()).toBe(BADGE_BASE_TITLE);
    expect(getBtnText()).not.toMatch(/·\s*\d/);
    vi.useRealTimers();
  });

  it('does not apply stale suffix when slow re-fetch completes after leaving corpus', async () => {
    vi.useFakeTimers();
    window.location.hash = '#/corpus/owner/repo';
    fetchKbDocCount.mockResolvedValue(3);

    initSedimentKbBadge();
    await vi.waitFor(() => {
      expect(getBtnText()).toBe(`${BADGE_BASE_TITLE} · 3`);
    });

    let resolveSlow;
    fetchKbDocCount.mockImplementation(
      () => new Promise((resolve) => { resolveSlow = resolve; }),
    );
    window.dispatchEvent(new CustomEvent('kb-diff-updated'));
    await vi.advanceTimersByTimeAsync(300);

    window.location.hash = '#/home';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    expect(getBtnText()).toBe(BADGE_BASE_TITLE);

    resolveSlow(88);
    await Promise.resolve();
    expect(getBtnText()).toBe(BADGE_BASE_TITLE);
    expect(getBtnText()).not.toMatch(/·\s*\d/);
    vi.useRealTimers();
  });

  it('restores base title when initial fetch rejects after 10s timeout', async () => {
    vi.useFakeTimers();
    window.location.hash = '#/corpus/owner/repo';
    fetchKbDocCount.mockImplementation(
      () =>
        new Promise((_, reject) => {
          setTimeout(() => reject(new Error('timeout')), 10_000);
        }),
    );

    initSedimentKbBadge();
    await Promise.resolve();
    expect(getBtnText()).toBe(BADGE_BASE_TITLE);

    await vi.advanceTimersByTimeAsync(10_000);
    await vi.waitFor(() => {
      expect(getBtnText()).toBe(BADGE_BASE_TITLE);
    });
    expect(getBtnText()).not.toMatch(/·\s*\d/);
    vi.useRealTimers();
  });

  it('clears stale suffix when re-fetch hangs past 10s timeout', async () => {
    vi.useFakeTimers();
    window.location.hash = '#/corpus/owner/repo';
    fetchKbDocCount.mockResolvedValueOnce(5);

    initSedimentKbBadge();
    await vi.waitFor(() => {
      expect(getBtnText()).toBe(`${BADGE_BASE_TITLE} · 5`);
    });

    fetchKbDocCount.mockImplementation(() => new Promise(() => {}));
    window.dispatchEvent(new CustomEvent('kb-diff-updated'));
    await vi.advanceTimersByTimeAsync(300);

    await vi.advanceTimersByTimeAsync(10_000);
    await Promise.resolve();

    expect(getBtnText()).toBe(BADGE_BASE_TITLE);
    expect(getBtnText()).not.toMatch(/·\s*\d/);
    vi.useRealTimers();
  });
});
