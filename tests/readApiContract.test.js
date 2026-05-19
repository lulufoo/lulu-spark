import { describe, expect, it } from 'vitest';
import {
  READ_API_INVOKE_MAP,
  normalizeForContract,
  resolveInvokeFromPath,
} from '../js/readApiInvokeMap.js';

describe('readApi contract map', () => {
  it('covers all P1 migrated GET paths', () => {
    const paths = [
      '/api/knowledge-index',
      '/api/search-knowledge',
      '/api/search-workbench',
      '/api/topics',
      '/api/annotations',
      '/api/annotation',
      '/api/draft',
      '/api/config',
      '/api/status',
      '/api/kb/read',
      '/api/kb/annotation',
      '/api/kb/status',
      '/api/repo-list',
      '/api/repo-dirs',
      '/api/check-file',
    ];
    for (const p of paths) {
      expect(READ_API_INVOKE_MAP[p]?.cmd, p).toBeTruthy();
    }
  });

  it('resolveInvokeFromPath maps query args', () => {
    expect(
      resolveInvokeFromPath('/api/kb/read?repo=o/r&path=docs/a.md')
    ).toEqual({
      cmd: 'kb_read',
      args: { repo: 'o/r', path: 'docs/a.md' },
    });
    expect(resolveInvokeFromPath('/api/repo-list?force=1')).toEqual({
      cmd: 'get_repo_list',
      args: { force: true },
    });
  });

  it('normalizeForContract drops cached_at', () => {
    const a = { entries: [], cached_at: 'x' };
    const b = { entries: [] };
    expect(normalizeForContract(a)).toEqual(normalizeForContract(b));
  });
});
