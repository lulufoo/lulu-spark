import { describe, expect, it } from 'vitest';
import {
  READ_API_INVOKE_MAP,
  normalizeForContract,
  resolveInvokeFromPath,
} from '../../frontend/src/host/readApiInvokeMap.ts';

describe('readApi contract map', () => {
  it('READ_API_INVOKE_MAP 的 path 与 cmd 一一对应（无遗漏路径）', () => {
    const paths = Object.keys(READ_API_INVOKE_MAP).sort();
    expect(paths.length).toBeGreaterThan(0);
    for (const p of paths) {
      expect(READ_API_INVOKE_MAP[p]?.cmd, p).toBeTruthy();
    }
  });

  it('resolveInvokeFromPath maps the search query with its workspace', () => {
    expect(resolveInvokeFromPath('/api/search-spark?q=abc&limit=8&workspace=memory')).toEqual({
      cmd: 'search_spark',
      args: { q: 'abc', workspace: 'memory', limit: 8 },
    });
    expect(resolveInvokeFromPath('/api/search-spark?q=abc')).toEqual({
      cmd: 'search_spark',
      args: { q: 'abc', workspace: undefined, limit: undefined },
    });
  });

  it('resolveInvokeFromPath no longer maps the knowledge routes', () => {
    for (const path of ['/api/kb/read?id=x', '/api/kb/list', '/api/sediment-kb/repos', '/api/search-knowledge?q=x', '/api/topics']) {
      expect(resolveInvokeFromPath(path), path).toBeNull();
    }
  });

  it('resolveInvokeFromPath 不再映射 infer-github-user-url', () => {
    expect(
      resolveInvokeFromPath(
        '/api/infer-github-user-url?path=%2FUsers%2Fme%2Fspark-knowledge',
      ),
    ).toBeNull();
    expect(
      resolveInvokeFromPath('/api/check-spark-root?path=%2Ftmp%2Fspark'),
    ).toBeNull();
  });

  it('normalizeForContract drops cached_at', () => {
    const a = { entries: [], cached_at: 'x' };
    const b = { entries: [] };
    expect(normalizeForContract(a)).toEqual(normalizeForContract(b));
  });

  it('resolveInvokeFromPath maps notes-asset query args', () => {
    expect(
      resolveInvokeFromPath(
        '/api/notes-asset?layer=raw&id=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa&href=note.png',
      ),
    ).toEqual({
      cmd: 'get_notes_asset',
      args: {
        layer: 'raw',
        id: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
        href: 'note.png',
        lang: undefined,
      },
    });
  });

  it('resolveInvokeFromPath maps read-later to get_read_later', () => {
    expect(resolveInvokeFromPath('/api/read-later')).toEqual({
      cmd: 'get_read_later',
      args: {},
    });
    expect(READ_API_INVOKE_MAP['/api/read-later']).toEqual({
      cmd: 'get_read_later',
    });
  });

  it('resolveInvokeFromPath maps doc-highlights to get_doc_highlights', () => {
    expect(
      resolveInvokeFromPath(
        '/api/doc-highlights?key=notes%3A20260827%2Fwhy-cache.md',
      ),
    ).toEqual({
      cmd: 'get_doc_highlights',
      args: { key: 'notes:20260827/why-cache.md' },
    });
  });

  it('resolveInvokeFromPath maps /api/file to read_abs_file', () => {
    expect(
      resolveInvokeFromPath('/api/file?path=%2Ftmp%2Fnotes%2Fraw%2Fx.md'),
    ).toEqual({
      cmd: 'read_abs_file',
      args: { path: '/tmp/notes/raw/x.md' },
    });
  });

  it('resolveInvokeFromPath no longer maps todo-tasks', () => {
    expect(resolveInvokeFromPath('/api/todo-tasks')).toBeNull();
    expect(READ_API_INVOKE_MAP['/api/todo-tasks']).toBeUndefined();
  });
});
