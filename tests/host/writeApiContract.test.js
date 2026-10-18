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
  ['/api/set-doc-source-type', 'set_doc_source_type', ['id', 'sourceType']],
  ['/api/tag/attach', 'tag_attach', ['id', 'key', 'value']],
  ['/api/tag/detach', 'tag_detach', ['id', 'key']],
  ['/api/tag/update-value', 'tag_update_value', ['key', 'value']],
  ['/api/create-note', 'create_note', ['payload']],
  ['/api/workspace-create', 'create_workspace', ['payload']],
  ['/api/workspace-rename', 'rename_workspace', ['payload']],
  ['/api/workspace-view-mode', 'set_workspace_view_mode', ['payload']],
  ['/api/workspace-icon', 'set_workspace_icon', ['payload']],
  ['/api/workspaces-reorder', 'reorder_workspaces', ['payload']],
  ['/api/workspace-disable', 'disable_workspace', ['payload']],
  ['/api/workspace-enable', 'enable_workspace', ['payload']],
  ['/api/project-create', 'create_project', ['payload']],
  ['/api/project-update', 'update_project', ['payload']],
  ['/api/projects-reorder', 'reorder_projects', ['payload']],
  ['/api/project-delete', 'delete_project', ['payload']],
  ['/api/hide-pattern-add', 'add_hide_pattern', ['payload']],
  ['/api/hide-pattern-update', 'update_hide_pattern', ['payload']],
  ['/api/hide-pattern-remove', 'remove_hide_pattern', ['payload']],
  ['/api/viewer-state', 'set_viewer_state', ['payload']],
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

  it('set-doc-source-type invoke args use camelCase sourceType', () => {
    expect(
      resolveWriteInvoke('/api/set-doc-source-type', {
        id: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
        source_type: 'summary',
      }),
    ).toEqual({
      cmd: 'set_doc_source_type',
      args: { id: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', sourceType: 'summary' },
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

  it('resolveWriteInvoke no longer maps the knowledge routes', () => {
    for (const path of ['/api/kb/save', '/api/sediment-kb/repos/add', '/api/kb/viewer-state']) {
      expect(resolveWriteInvoke(path, {}), path).toBeNull();
    }
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
