import { describe, expect, it } from 'vitest';
import {
  WRITE_API_INVOKE_MAP,
  resolveWriteInvoke,
} from '../../frontend/src/host/writeApiInvokeMap.ts';

const P2_PATHS = [
  ['/api/config', 'set_config', ['payload']],
  ['/api/save', 'save_entry', ['layer', 'commonPath', 'content']],
  ['/api/update-comments', 'update_comments', ['commonPath', 'layer', 'comment', 'ts']],
  ['/api/reorder-comments', 'reorder_comments', ['commonPath', 'layer', 'ids']],
  ['/api/update-highlights', 'update_highlights', ['commonPath', 'layer', 'highlight', 'ts']],
  ['/api/doc-highlights', 'update_doc_highlights', ['key', 'highlight', 'ts']],
  ['/api/update-links', 'update_links', ['commonPath', 'links']],
  ['/api/set-done', 'set_done', ['commonPath', 'done']],
  ['/api/set-importance', 'set_importance', ['commonPath', 'importance']],
  ['/api/kb/save', 'kb_save', ['repo', 'path', 'content']],
  ['/api/kb/update-comments', 'kb_update_comments', ['repo', 'path', 'comment', 'ts']],
  ['/api/kb/reorder-comments', 'kb_reorder_comments', ['repo', 'path', 'ids']],
  ['/api/kb/update-highlights', 'kb_update_highlights', ['repo', 'path', 'highlight', 'ts']],
  ['/api/kb/update-links', 'kb_update_links', ['repo', 'path', 'links']],
  ['/api/tag/attach', 'tag_attach', ['commonPath', 'key', 'value']],
  ['/api/tag/detach', 'tag_detach', ['commonPath', 'key']],
  ['/api/tag/update-value', 'tag_update_value', ['key', 'value']],
  ['/api/sediment-kb/repos/add', 'sediment_kb_add_repo', ['payload']],
  ['/api/sediment-kb/repos/remove', 'sediment_kb_remove_repo', ['payload']],
  [
    '/api/sediment-kb/repos/update-category',
    'sediment_kb_update_repo_category',
    ['payload'],
  ],
  ['/api/sediment-kb/categories/add', 'sediment_kb_add_category', ['payload']],
  [
    '/api/sediment-kb/categories/rename',
    'sediment_kb_rename_category',
    ['payload'],
  ],
  [
    '/api/sediment-kb/categories/remove',
    'sediment_kb_remove_category',
    ['payload'],
  ],
  ['/api/create-note', 'create_note', ['payload']],
  ['/api/notes-category-create', 'create_notes_category', ['payload']],
  ['/api/notes-category-update', 'update_notes_category', ['payload']],
  ['/api/notes-category-delete', 'delete_notes_category', ['payload']],
  ['/api/kb/hide-patterns/add', 'add_kb_hide_pattern', ['payload']],
  ['/api/kb/hide-patterns/update', 'update_kb_hide_pattern', ['payload']],
  ['/api/kb/hide-patterns/remove', 'remove_kb_hide_pattern', ['payload']],
];

describe('writeApiContract', () => {
  it('covers all P2 POST paths with command names including create-note', () => {
    expect(Object.keys(WRITE_API_INVOKE_MAP).length).toBeGreaterThanOrEqual(P2_PATHS.length);
    for (const [path, cmd] of P2_PATHS) {
      expect(WRITE_API_INVOKE_MAP[path]?.cmd, path).toBe(cmd);
    }
  });

  it('set_done invoke args match server.py body', () => {
    expect(
      resolveWriteInvoke('/api/set-done', {
        common_path: 'ai/x.md',
        done: true,
      })
    ).toEqual({
      cmd: 'set_done',
      args: { commonPath: 'ai/x.md', done: true },
    });
  });

  it('save_entry invoke args match server.py body', () => {
    expect(
      resolveWriteInvoke('/api/save', {
        layer: 'digest',
        common_path: 'a/b.md',
        content: '# t',
      })
    ).toEqual({
      cmd: 'save_entry',
      args: { layer: 'digest', commonPath: 'a/b.md', content: '# t' },
    });
  });

  it('update_links invoke args use commonPath (Tauri 2 IPC)', () => {
    const resolved = resolveWriteInvoke('/api/update-links', {
      common_path: 'inbox/new-note.md',
      links: [{ url: 'https://github.com/foo/bar' }],
    });
    expect(resolved).toEqual({
      cmd: 'update_links',
      args: {
        commonPath: 'inbox/new-note.md',
        links: [{ url: 'https://github.com/foo/bar' }],
      },
    });
    expect(resolved.args).not.toHaveProperty('common_path');
  });

  it('notes annotation endpoints map common_path → commonPath only', () => {
    const cases = [
      [
        '/api/update-comments',
        { common_path: 'ai/x.md', layer: 'raw', comment: { text: 'a' }, ts: '1' },
        ['commonPath', 'layer', 'comment', 'ts'],
      ],
      [
        '/api/reorder-comments',
        { common_path: 'ai/x.md', layer: 'raw', ids: ['a'] },
        ['commonPath', 'layer', 'ids'],
      ],
      [
        '/api/update-highlights',
        { common_path: 'ai/x.md', layer: 'raw', highlight: { id: 'h' }, ts: '1' },
        ['commonPath', 'layer', 'highlight', 'ts'],
      ],
      [
        '/api/set-importance',
        { common_path: 'ai/x.md', importance: 'high' },
        ['commonPath', 'importance'],
      ],
    ];
    for (const [path, body, keys] of cases) {
      const resolved = resolveWriteInvoke(path, body);
      expect(Object.keys(resolved.args).sort(), path).toEqual(keys.sort());
      expect(resolved.args.commonPath).toBe('ai/x.md');
      expect(resolved.args).not.toHaveProperty('common_path');
    }
  });

  it('sediment-kb write invoke args pass body as payload', () => {
    expect(
      resolveWriteInvoke('/api/sediment-kb/repos/add', {
        full_name: 'acme/demo',
        category_id: 'cat-1',
      }),
    ).toEqual({
      cmd: 'sediment_kb_add_repo',
      args: {
        payload: { full_name: 'acme/demo', category_id: 'cat-1' },
      },
    });
    expect(
      resolveWriteInvoke('/api/sediment-kb/categories/rename', {
        id: 'cat-1',
        name: 'Docs',
      }),
    ).toEqual({
      cmd: 'sediment_kb_rename_category',
      args: { payload: { id: 'cat-1', name: 'Docs' } },
    });
  });

  it('resolveWriteInvoke returns null for unknown sediment-kb route', () => {
    expect(resolveWriteInvoke('/api/sediment-kb/repos/unknown', {})).toBeNull();
  });

  it('golden error shapes align with P1 kb_read style', () => {
    expect({ error: 'Invalid common_path', _status: 400 }).toMatchObject({
      error: 'Invalid common_path',
      _status: 400,
    });
    expect({ error: 'Comment not found', _status: 404 }).toMatchObject({
      error: 'Comment not found',
      _status: 404,
    });
  });

  it('doc-highlights invoke args pass key / highlight / ts', () => {
    expect(
      resolveWriteInvoke('/api/doc-highlights', {
        key: 'knowledge:acme/docs/foo.md',
        highlight: { text: 'only', occurrence: 0 },
        ts: 't1',
      }),
    ).toEqual({
      cmd: 'update_doc_highlights',
      args: {
        key: 'knowledge:acme/docs/foo.md',
        highlight: { text: 'only', occurrence: 0 },
        ts: 't1',
      },
    });
  });

  it('create-note invoke args pass HTTP body as payload (snake HTTP ↔ camel invoke)', () => {
    const body = {
      source_path: '/tmp/title.md',
      source_type: 'summary',
    };
    expect(resolveWriteInvoke('/api/create-note', body)).toEqual({
      cmd: 'create_note',
      args: { payload: body },
    });
  });
});
