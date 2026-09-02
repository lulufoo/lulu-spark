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

  it('resolveInvokeFromPath maps kb list query args', () => {
    expect(
      resolveInvokeFromPath('/api/kb/list?repo=o/r&path=&mode=flat'),
    ).toEqual({
      cmd: 'kb_list',
      args: { repo: 'o/r', path: '', mode: 'flat' },
    });
  });

  it('READ_API_INVOKE_MAP maps /api/kb/doc-count to kb_doc_count', () => {
    const entry = READ_API_INVOKE_MAP['/api/kb/doc-count'];
    expect(entry?.cmd).toBe('kb_doc_count');
    expect(typeof entry?.args).toBe('function');
  });

  it('resolveInvokeFromPath maps kb doc-count query args', () => {
    expect(
      resolveInvokeFromPath('/api/kb/doc-count?repo=o/r&hide_pattern=^draft'),
    ).toEqual({
      cmd: 'kb_doc_count',
      args: { repo: 'o/r', hide_pattern: '^draft', category_id: undefined },
    });
  });

  it('resolveInvokeFromPath maps kb doc-count with repo-only query', () => {
    const result = resolveInvokeFromPath('/api/kb/doc-count?repo=o/r');
    expect(result).toEqual({
      cmd: 'kb_doc_count',
      args: { repo: 'o/r', hide_pattern: undefined, category_id: undefined },
    });
  });

  it('resolveInvokeFromPath decodes URL-encoded repo for kb doc-count', () => {
    expect(
      resolveInvokeFromPath('/api/kb/doc-count?repo=o%2Fr&hide_pattern=^draft'),
    ).toEqual({
      cmd: 'kb_doc_count',
      args: { repo: 'o/r', hide_pattern: '^draft', category_id: undefined },
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
  });

  it('resolveInvokeFromPath maps notes-asset query args', () => {
    expect(
      resolveInvokeFromPath(
        '/api/notes-asset?layer=raw&base=ai/note.md&href=note.png',
      ),
    ).toEqual({
      cmd: 'get_notes_asset',
      args: { layer: 'raw', base: 'ai/note.md', href: 'note.png' },
    });
  });

  it('resolveInvokeFromPath returns null for unknown sediment-kb route', () => {
    expect(resolveInvokeFromPath('/api/sediment-kb/unknown')).toBeNull();
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

  it('resolveInvokeFromPath maps todo-tasks to get_todo_tasks', () => {
    expect(resolveInvokeFromPath('/api/todo-tasks')).toEqual({
      cmd: 'get_todo_tasks',
      args: {},
    });
    expect(READ_API_INVOKE_MAP['/api/todo-tasks']).toEqual({
      cmd: 'get_todo_tasks',
    });
  });
});
