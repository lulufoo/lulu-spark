import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it, vi } from 'vitest';
import { resolveInvokeFromPath } from '../../frontend/src/host/readApiInvokeMap.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

describe('board sketch read bridge', () => {
  it.each([
    { status: 'cache_miss' },
    {
      status: 'sketch', content_b64: 'PDw+Pz89', version: 7,
      credential_scope: 'opaque-scope', generation: 2,
      diagram_id: 'b_0123abcd', diagram_generation: 0,
    },
  ])('reads $status through the cache-only command without a remote read', async (reply) => {
    const page = 'https://luluboard.app/view?utm=原始#b:b_0123abcd';
    expect(resolveInvokeFromPath(`/api/board-sketch-cache?url=${encodeURIComponent(page)}`)).toEqual({
      cmd: 'read_cached_board_sketch',
      args: { url: page },
    });
    const api = await import('../../frontend/src/host/api.ts');
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200, json: async () => reply, text: async () => '',
    });
    expect(await api.fetchCachedBoardSketch(page)).toEqual(reply);
    expect(globalThis.fetch).toHaveBeenCalledTimes(1);
    const called = new URL(globalThis.fetch.mock.calls[0][0], 'http://localhost');
    expect(called.pathname).toBe('/api/board-sketch-cache');
    expect(called.searchParams.get('url')).toBe(page);
  });

  it('passes the page url to the read command and does not store a token', async () => {
    const page = 'https://luluboard.app/view?utm=1#b:b_0123abcd';
    expect(resolveInvokeFromPath(`/api/board-sketch?url=${encodeURIComponent(page)}`)).toEqual({
      cmd: 'read_board_sketch',
      args: { url: page },
    });
    const src = readFileSync(join(repoRoot, 'frontend/src/host/api/board-sketch.ts'), 'utf8');
    expect(src).not.toMatch(/localStorage|sessionStorage|pat/);
    const api = await import('../../frontend/src/host/api.ts');
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ status: 'sketch', content_b64: 'PDw+Pz89', title: 'Title', version: 7 }),
      text: async () => '',
    });
    expect(await api.fetchBoardSketch(page)).toEqual({
      status: 'sketch', content_b64: 'PDw+Pz89', title: 'Title', version: 7,
    });
    const called = globalThis.fetch.mock.calls[0][0];
    expect(called).toContain('/api/board-sketch?url=');
    expect(called).toContain(encodeURIComponent(page));
    expect(called).not.toContain('lbp_');
    expect(globalThis.fetch).toHaveBeenCalledTimes(1);
  });

  it.each([
    { status: 'unauthorized', invalidation_scope: 'credential' },
    { status: 'forbidden', invalidation_scope: 'diagram' },
    { status: 'not_found', invalidation_scope: 'diagram' },
    { status: 'unconfigured', invalidation_scope: 'credential' },
    { status: 'unavailable' },
    { status: 'network' },
    { status: 'invalid_document' },
    { status: 'not_sketch', version: 2 },
    { status: 'invalid_source', version: 3 },
  ])('preserves the host result and original URL for $status', async (reply) => {
    const page = 'https://luluboard.app/view?utm=原始#b:b_0123abcd';
    const api = await import('../../frontend/src/host/api.ts');
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => reply,
      text: async () => '',
    });
    expect(await api.fetchBoardSketch(page)).toEqual(reply);
    expect(globalThis.fetch).toHaveBeenCalledTimes(1);
    const called = new URL(globalThis.fetch.mock.calls[0][0], 'http://localhost');
    expect(called.searchParams.get('url')).toBe(page);
  });
});
