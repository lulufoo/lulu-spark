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
    expect(resolveInvokeFromPath('/api/search-spark?q=abc&limit=8&workspace_id=memory')).toEqual({
      cmd: 'search_spark',
      args: { q: 'abc', workspaceId: 'memory', limit: 8 },
    });
    expect(resolveInvokeFromPath('/api/search-spark?q=abc')).toEqual({
      cmd: 'search_spark',
      args: { q: 'abc', workspaceId: undefined, limit: undefined },
    });
  });

  it('resolveInvokeFromPath maps the disabled workspace list and no longer a project one', () => {
    expect(resolveInvokeFromPath('/api/workspaces-disabled?_=1')).toEqual({
      cmd: 'list_disabled_workspaces',
      args: {},
    });
    expect(resolveInvokeFromPath('/api/projects-disabled?workspace_id=ws1&_=1')).toBeNull();
  });

  it('resolveInvokeFromPath maps the per-workspace hidden-files rules and viewer state', () => {
    expect(resolveInvokeFromPath('/api/hide-patterns?workspace_id=ws1&_=1')).toEqual({
      cmd: 'list_hide_patterns',
      args: { payload: { workspace_id: 'ws1' } },
    });
    expect(resolveInvokeFromPath('/api/viewer-state?workspace_id=ws1&_=1')).toEqual({
      cmd: 'get_viewer_state',
      args: { payload: { workspace_id: 'ws1' } },
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

  it('resolveInvokeFromPath maps doc-asset query args', () => {
    expect(
      resolveInvokeFromPath(
        '/api/doc-asset?layer=raw&id=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa&href=note.png',
      ),
    ).toEqual({
      cmd: 'get_doc_asset',
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
        '/api/doc-highlights?key=doc%3A20260827%2Fwhy-cache.md',
      ),
    ).toEqual({
      cmd: 'get_doc_highlights',
      args: { key: 'doc:20260827/why-cache.md' },
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
