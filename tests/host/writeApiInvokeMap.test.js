import { describe, expect, it, vi } from 'vitest';
import {
  WRITE_API_INVOKE_MAP,
  resolveWriteInvoke,
} from '../../frontend/src/host/writeApiInvokeMap.ts';

const P2_WRITE_PATHS = [
  '/api/config',
  '/api/save',
  '/api/update-comments',
  '/api/reorder-comments',
  '/api/update-highlights',
  '/api/doc-highlights',
  '/api/update-links',
  '/api/set-done',
  '/api/set-importance',
  '/api/read-later',
  '/api/kb/save',
  '/api/kb/rename',
  '/api/kb/update-comments',
  '/api/kb/reorder-comments',
  '/api/kb/update-highlights',
  '/api/kb/update-links',
  '/api/tag/attach',
  '/api/tag/detach',
  '/api/tag/update-value',
  '/api/sediment-kb/repos/add',
  '/api/sediment-kb/repos/remove',
  '/api/sediment-kb/repos/update-category',
  '/api/sediment-kb/categories/add',
  '/api/sediment-kb/categories/rename',
  '/api/sediment-kb/categories/remove',
  '/api/create-note',
  '/api/note-draft',
  '/api/note-draft/clear',
  '/api/notes-category-create',
  '/api/notes-category-update',
  '/api/notes-category-delete',
  '/api/mark-message-channel-read',
  '/api/kb/hide-patterns/add',
  '/api/kb/hide-patterns/update',
  '/api/kb/hide-patterns/remove',
  '/api/kb/viewer-state',
];

const ANNOTATION_WRITE_CMDS = new Set([
  'update_comments',
  'reorder_comments',
  'update_highlights',
  'update_links',
  'set_done',
  'set_importance',
]);

describe('writeApiInvokeMap', () => {
  it('covers all P2 POST paths', () => {
    for (const p of P2_WRITE_PATHS) {
      expect(WRITE_API_INVOKE_MAP[p]?.cmd, p).toBeTruthy();
    }
    expect(Object.keys(WRITE_API_INVOKE_MAP)).toHaveLength(37);
  });

  it('maps /api/file absolute-path write to write_abs_file', () => {
    expect(WRITE_API_INVOKE_MAP['/api/file']?.cmd).toBe('write_abs_file');
    expect(
      resolveWriteInvoke('/api/file', {
        path: '/tmp/notes/raw/x.md',
        content: '# hi',
      }),
    ).toEqual({
      cmd: 'write_abs_file',
      args: { path: '/tmp/notes/raw/x.md', content: '# hi' },
    });
  });

  it('resolveWriteInvoke maps set-done body to set_done command', () => {
    expect(
      resolveWriteInvoke('/api/set-done', {
        common_path: 'ai/note.md',
        done: true,
      })
    ).toEqual({
      cmd: 'set_done',
      args: { commonPath: 'ai/note.md', done: true },
    });
  });

  it('resolveWriteInvoke returns null for unknown path', () => {
    expect(resolveWriteInvoke('/api/commit', {})).toBeNull();
    expect(resolveWriteInvoke('/api/sediment-kb/unknown', {})).toBeNull();
  });

  it('resolveWriteInvoke maps save_entry fields', () => {
    expect(
      resolveWriteInvoke('/api/save', {
        layer: 'digest',
        common_path: 'x/y.md',
        content: '# hi',
      })
    ).toEqual({
      cmd: 'save_entry',
      args: { layer: 'digest', commonPath: 'x/y.md', content: '# hi' },
    });
  });

  it('resolveWriteInvoke maps read-later body to create_read_later command', () => {
    expect(
      resolveWriteInvoke('/api/read-later', {
        url: 'https://example.com/a',
        title: 'Example',
      })
    ).toEqual({
      cmd: 'create_read_later',
      args: { url: 'https://example.com/a', title: 'Example' },
    });
  });

  it('resolveWriteInvoke maps read-later without title', () => {
    expect(
      resolveWriteInvoke('/api/read-later', {
        url: 'https://example.com/b',
      })
    ).toEqual({
      cmd: 'create_read_later',
      args: { url: 'https://example.com/b' },
    });
  });

  it('resolveWriteInvoke maps create-note to create_note (snake↔camel style)', () => {
    const body = {
      source_path: '/tmp/note.md',
      source_type: 'summary',
    };
    expect(resolveWriteInvoke('/api/create-note', body)).toEqual({
      cmd: 'create_note',
      args: { payload: body },
    });
  });

  it('resolveWriteInvoke maps doc-highlights to update_doc_highlights', () => {
    expect(
      resolveWriteInvoke('/api/doc-highlights', {
        key: 'notes:20260827/why-cache.md',
        highlight: { text: 'cache', occurrence: 0 },
        ts: '20260827T154000',
      }),
    ).toEqual({
      cmd: 'update_doc_highlights',
      args: {
        key: 'notes:20260827/why-cache.md',
        highlight: { text: 'cache', occurrence: 0 },
        ts: '20260827T154000',
      },
    });
  });

  it('doc-highlights invoke does not route to Annotation write commands', () => {
    const resolved = resolveWriteInvoke('/api/doc-highlights', {
      key: 'todos:task_1',
      highlight: { id: 'h1' },
      ts: 't',
    });
    expect(resolved).not.toBeNull();
    expect(ANNOTATION_WRITE_CMDS.has(resolved.cmd)).toBe(false);
    expect(resolved.cmd).toBe('update_doc_highlights');
  });

  it('create-note invoke does not route to Annotation write commands', () => {
    const resolved = resolveWriteInvoke('/api/create-note', {
      source_path: '/tmp/note.md',
      source_type: 'summary',
    });
    expect(resolved).not.toBeNull();
    expect(ANNOTATION_WRITE_CMDS.has(resolved.cmd)).toBe(false);
    expect(resolved.cmd).toBe('create_note');
  });

  it('WRITE_API_INVOKE_MAP 含 mark_message_channel_read', () => {
    expect(WRITE_API_INVOKE_MAP['/api/mark-message-channel-read']?.cmd).toBe(
      'mark_message_channel_read',
    );
  });

  it('resolveWriteInvoke maps mark-message-channel-read body channel (snake HTTP → camel invoke)', () => {
    expect(
      resolveWriteInvoke('/api/mark-message-channel-read', {
        channel: 'notes',
      }),
    ).toEqual({
      cmd: 'mark_message_channel_read',
      args: { channel: 'notes' },
    });
  });

  it('host/api.ts 对外暴露 markMessageChannelRead，传输走 transport.ts', async () => {
    const api = await import('../../frontend/src/host/api.ts');
    expect(typeof api.markMessageChannelRead).toBe('function');
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({}),
      text: async () => '{}',
    });
    await api.markMessageChannelRead('notes');
    expect(globalThis.fetch.mock.calls[0][0]).toMatch(/mark-message-channel-read/);
    expect(globalThis.fetch.mock.calls[0][1].method).toBe('POST');
    expect(JSON.parse(globalThis.fetch.mock.calls[0][1].body)).toEqual({
      channel: 'notes',
    });
  });
});
