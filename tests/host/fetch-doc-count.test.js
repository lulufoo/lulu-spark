import { beforeEach, describe, expect, it, vi } from 'vitest';
import { DEFAULT_DEV_BASE } from '../../frontend/src/host/apiClient.ts';
import { fetchKbDocCount } from '../../frontend/src/host/api.ts';

const API_READ_PREFIX = `${DEFAULT_DEV_BASE}/api`;

function mockFetch(body, ok = true, status = 200) {
  globalThis.fetch = vi.fn().mockResolvedValue({
    ok,
    status,
    json: () => Promise.resolve(body),
    text: () => Promise.resolve(typeof body === 'string' ? body : JSON.stringify(body)),
  });
}

function mockSlowFetch(delayMs = 15_000) {
  globalThis.fetch = vi.fn().mockImplementation(
    () =>
      new Promise((resolve) => {
        setTimeout(
          () =>
            resolve({
              ok: true,
              status: 200,
              json: () => Promise.resolve({ count: 1 }),
              text: () => Promise.resolve(JSON.stringify({ count: 1 })),
            }),
          delayMs,
        );
      }),
  );
}

beforeEach(() => {
  vi.restoreAllMocks();
});

describe('fetchKbDocCount', () => {
  it('calls GET /api/kb/doc-count with repo only', async () => {
    mockFetch({ count: 42 });

    const count = await fetchKbDocCount('owner/repo');

    expect(fetch).toHaveBeenCalledOnce();
    expect(fetch.mock.calls[0][0]).toMatch(
      new RegExp(`^${API_READ_PREFIX}/kb/doc-count`),
    );
    const url = new URL(fetch.mock.calls[0][0]);
    expect(url.searchParams.get('repo')).toBe('owner/repo');
    expect(url.searchParams.has('hide_pattern')).toBe(false);
    expect(count).toBe(42);
  });

  it('returns count number on success', async () => {
    mockFetch({ count: 7 });
    await expect(fetchKbDocCount('o/r')).resolves.toBe(7);
  });

  it('returns 0 when count is 0', async () => {
    mockFetch({ count: 0 });
    await expect(fetchKbDocCount('o/r')).resolves.toBe(0);
  });

  it('throws Error on HTTP 4xx/5xx', async () => {
    mockFetch({ error: 'repo not found' }, false, 404);
    await expect(fetchKbDocCount('missing/repo')).rejects.toThrow('repo not found');
  });

  it('throws Error when JSON body contains error', async () => {
    mockFetch({ error: 'invalid hide pattern' });
    await expect(fetchKbDocCount('o/r')).rejects.toThrow('invalid hide pattern');
  });

  it('throws when request exceeds 10s (AbortController timeout)', async () => {
    vi.useFakeTimers();
    mockSlowFetch(15_000);

    const pending = fetchKbDocCount('o/r');
    const assertion = expect(pending).rejects.toThrow();

    await vi.advanceTimersByTimeAsync(10_000);

    await assertion;
    vi.useRealTimers();
  });
});
