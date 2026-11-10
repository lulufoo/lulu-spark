import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it, vi } from 'vitest';
import { resolveWriteInvoke } from '../../frontend/src/host/writeApiInvokeMap.ts';
import { resolveInvokeFromPath } from '../../frontend/src/host/readApiInvokeMap.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

describe('board pat bridge', () => {
  it('maps status to the read command and writes only the pat argument', () => {
    expect(resolveInvokeFromPath('/api/board-pat')).toEqual({
      cmd: 'get_board_pat_status',
      args: {},
    });
    expect(resolveWriteInvoke('/api/board-pat', { pat: 'lin_pat_x' })).toEqual({
      cmd: 'set_board_pat',
      args: { pat: 'lin_pat_x' },
    });
    expect(resolveWriteInvoke('/api/board-pat/delete', {})).toEqual({
      cmd: 'delete_board_pat',
      args: {},
    });
  });

  it('host api forwards status, replace, and delete without storing the token', async () => {
    const src = readFileSync(join(repoRoot, 'frontend/src/host/api/board-pat.ts'), 'utf8');
    expect(src).not.toMatch(/localStorage|sessionStorage/);
    const api = await import('../../frontend/src/host/api.ts');
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => true,
      text: async () => 'true',
    });
    await api.fetchBoardPatStatus();
    expect(globalThis.fetch.mock.calls[0][0]).toMatch(/\/api\/board-pat$/);
    await api.setBoardPat('lin_pat_x');
    expect(JSON.parse(globalThis.fetch.mock.calls[1][1].body)).toEqual({ pat: 'lin_pat_x' });
    await api.deleteBoardPat();
    expect(globalThis.fetch.mock.calls[2][0]).toMatch(/\/api\/board-pat\/delete$/);
  });
});
