import { describe, expect, it } from 'vitest';
import {
  fetchSedimentKbDocs,
  fetchSedimentKbRepos,
} from '../frontend/js/api.js';
import {
  READ_API_INVOKE_MAP,
  normalizeForContract,
  resolveInvokeFromPath,
} from '../frontend/js/readApiInvokeMap.js';

const SEDIMENT_KB_READ_ROUTES = [
  ['/api/sediment-kb/categories', 'get_sediment_kb_categories'],
  ['/api/sediment-kb/repos', 'get_sediment_kb_repos'],
  ['/api/sediment-kb/docs', 'get_sediment_kb_docs'],
];

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

  it('resolveInvokeFromPath maps sediment-kb read routes', () => {
    expect(resolveInvokeFromPath('/api/sediment-kb/categories')).toEqual({
      cmd: 'get_sediment_kb_categories',
      args: {},
    });
    expect(resolveInvokeFromPath('/api/sediment-kb/repos?_=' + Date.now())).toEqual({
      cmd: 'get_sediment_kb_repos',
      args: {},
    });
    expect(resolveInvokeFromPath('/api/sediment-kb/docs?repo=owner%2Fname')).toEqual({
      cmd: 'get_sediment_kb_docs',
      args: { repo: 'owner/name' },
    });
  });

  it('covers all sediment-kb read routes in the invoke map', () => {
    for (const [path, cmd] of SEDIMENT_KB_READ_ROUTES) {
      expect(READ_API_INVOKE_MAP[path]?.cmd, path).toBe(cmd);
    }
  });

  it('maps sediment-kb docs with an explicit repo argument', () => {
    expect(resolveInvokeFromPath('/api/sediment-kb/docs?repo=')).toEqual({
      cmd: 'get_sediment_kb_docs',
      args: { repo: '' },
    });
  });

  it('api.js exposes sediment-kb docs reader', () => {
    expect(typeof fetchSedimentKbRepos).toBe('function');
    expect(typeof fetchSedimentKbDocs).toBe('function');
  });

  it('resolveInvokeFromPath maps corpus-asset query args', () => {
    expect(
      resolveInvokeFromPath(
        '/api/corpus-asset?layer=raw&base=ai/note.md&href=note.png',
      ),
    ).toEqual({
      cmd: 'get_corpus_asset',
      args: { layer: 'raw', base: 'ai/note.md', href: 'note.png' },
    });
  });

  it('resolveInvokeFromPath returns null for unknown sediment-kb route', () => {
    expect(resolveInvokeFromPath('/api/sediment-kb/unknown')).toBeNull();
  });
});
