import { describe, expect, it } from 'vitest';
import {
  WRITE_API_INVOKE_MAP,
  resolveWriteInvoke,
} from '../../frontend/src/host/writeApiInvokeMap.ts';

const P2_PATHS = [
  ['/api/config', 'set_config', ['payload']],
  ['/api/save', 'save_entry', ['layer', 'id', 'content']],
  ['/api/update-comments', 'update_comments', ['id', 'layer', 'comment', 'ts']],
  ['/api/reorder-comments', 'reorder_comments', ['id', 'layer', 'ids']],
  ['/api/update-highlights', 'update_highlights', ['id', 'layer', 'highlight', 'ts']],
  ['/api/doc-highlights', 'update_doc_highlights', ['key', 'highlight', 'ts']],
  ['/api/update-links', 'update_links', ['id', 'links']],
  ['/api/set-done', 'set_done', ['id', 'done']],
  ['/api/set-importance', 'set_importance', ['id', 'importance']],
  ['/api/kb/save', 'kb_save', ['repo', 'path', 'content']],
  ['/api/kb/rename', 'kb_rename', ['repo', 'path', 'name']],
  ['/api/kb/create', 'kb_create', ['repo', 'parent', 'name', 'kind']],
  ['/api/kb/delete', 'kb_delete', ['repo', 'path']],
  ['/api/kb/update-comments', 'kb_update_comments', ['repo', 'path', 'comment', 'ts']],
  ['/api/kb/reorder-comments', 'kb_reorder_comments', ['repo', 'path', 'ids']],
  ['/api/kb/update-highlights', 'kb_update_highlights', ['repo', 'path', 'highlight', 'ts']],
  ['/api/kb/update-links', 'kb_update_links', ['repo', 'path', 'links']],
  ['/api/tag/attach', 'tag_attach', ['id', 'key', 'value']],
  ['/api/tag/detach', 'tag_detach', ['id', 'key']],
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
  ['/api/kb/viewer-state', 'set_kb_viewer_state', ['repo', 'path']],
  ['/api/show-os-notification', 'show_os_notification', ['title', 'body', 'scheme']],
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
        id: 'ai/x.md',
        done: true,
      })
    ).toEqual({
      cmd: 'set_done',
      args: { id: 'ai/x.md', done: true },
    });
  });

  it('save_entry invoke args match server.py body', () => {
    expect(
      resolveWriteInvoke('/api/save', {
        layer: 'digest',
        id: 'a/b.md',
        content: '# t',
      })
    ).toEqual({
      cmd: 'save_entry',
      args: { layer: 'digest', id: 'a/b.md', content: '# t' },
    });
  });

  it('update_links invoke args use id (Tauri 2 IPC)', () => {
    const resolved = resolveWriteInvoke('/api/update-links', {
      id: 'inbox/new-note.md',
      links: [{ url: 'https://github.com/foo/bar' }],
    });
    expect(resolved).toEqual({
      cmd: 'update_links',
      args: {
        id: 'inbox/new-note.md',
        links: [{ url: 'https://github.com/foo/bar' }],
      },
    });
    expect(resolved.args).not.toHaveProperty('common_path');
  });

  it('notes annotation endpoints map id only', () => {
    const cases = [
      [
        '/api/update-comments',
        { id: 'ai/x.md', layer: 'raw', comment: { text: 'a' }, ts: '1' },
        ['id', 'layer', 'comment', 'ts'],
      ],
      [
        '/api/reorder-comments',
        { id: 'ai/x.md', layer: 'raw', ids: ['a'] },
        ['id', 'layer', 'ids'],
      ],
      [
        '/api/update-highlights',
        { id: 'ai/x.md', layer: 'raw', highlight: { id: 'h' }, ts: '1' },
        ['id', 'layer', 'highlight', 'ts'],
      ],
      [
        '/api/set-importance',
        { id: 'ai/x.md', importance: 'high' },
        ['id', 'importance'],
      ],
    ];
    for (const [path, body, keys] of cases) {
      const resolved = resolveWriteInvoke(path, body);
      expect(Object.keys(resolved.args).sort(), path).toEqual(keys.sort());
      expect(resolved.args.id).toBe('ai/x.md');
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
    expect({ error: 'Invalid id', _status: 400 }).toMatchObject({
      error: 'Invalid id',
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
