import { describe, expect, it } from 'vitest';
import {
  READ_API_INVOKE_MAP,
  normalizeForContract,
  resolveInvokeFromPath,
} from '../frontend/js/readApiInvokeMap.js';

describe('readApi contract map', () => {
  it('READ_API_INVOKE_MAP 的 path 与 cmd 一一对应（无遗漏路径）', () => {
    const paths = Object.keys(READ_API_INVOKE_MAP).sort();
    expect(paths.length).toBeGreaterThan(0);
    for (const p of paths) {
      expect(READ_API_INVOKE_MAP[p]?.cmd, p).toBeTruthy();
    }
  });

  it('resolveInvokeFromPath 映射 infer-github-user-url', () => {
    expect(
      resolveInvokeFromPath(
        '/api/infer-github-user-url?path=%2FUsers%2Fme%2Fworkbench-knowledge',
      ),
    ).toEqual({
      cmd: 'infer_github_user_url',
      args: { path: '/Users/me/workbench-knowledge' },
    });
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

  it('resolveInvokeFromPath 映射 kb diff status', () => {
    expect(resolveInvokeFromPath('/api/kb/diff-status?_=123')).toEqual({
      cmd: 'get_kb_diff_status',
      args: {},
    });
  });

  it('normalizeForContract drops cached_at', () => {
    const a = { entries: [], cached_at: 'x' };
    const b = { entries: [] };
    expect(normalizeForContract(a)).toEqual(normalizeForContract(b));
  });
});
