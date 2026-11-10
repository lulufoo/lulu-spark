import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it, vi } from 'vitest';
import { resolveInvokeFromPath } from '../../frontend/src/host/readApiInvokeMap.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

describe('board sketch read bridge', () => {
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
      json: async () => ({ status: 'sketch', content_b64: 'PDw+Pz89' }),
      text: async () => '',
    });
    await api.fetchBoardSketch(page);
    const called = globalThis.fetch.mock.calls[0][0];
    expect(called).toContain('/api/board-sketch?url=');
    expect(called).toContain(encodeURIComponent(page));
    expect(called).not.toContain('lbp_');
  });
});
