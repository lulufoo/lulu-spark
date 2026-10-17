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
  '/api/tag/attach',
  '/api/tag/detach',
  '/api/tag/update-value',
  '/api/create-note',
  '/api/doc-draft',
  '/api/doc-draft/clear',
  '/api/workspace-create',
  '/api/workspace-rename',
  '/api/workspace-view-mode',
  '/api/workspace-icon',
  '/api/workspaces-reorder',
  '/api/workspace-disable',
  '/api/workspace-enable',
  '/api/project-create',
  '/api/project-update',
  '/api/projects-reorder',
  '/api/project-disable',
  '/api/project-enable',
  '/api/hide-pattern-add',
  '/api/hide-pattern-update',
  '/api/hide-pattern-remove',
  '/api/viewer-state',
  '/api/mark-message-channel-read',
  '/api/show-os-notification',
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
    expect(Object.keys(WRITE_API_INVOKE_MAP)).toHaveLength(35);
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
        id: 'ai/note.md',
        done: true,
      })
    ).toEqual({
      cmd: 'set_done',
      args: { id: 'ai/note.md', done: true },
    });
  });

  it('resolveWriteInvoke returns null for unknown path', () => {
    expect(resolveWriteInvoke('/api/commit', {})).toBeNull();
    expect(resolveWriteInvoke('/api/kb/unknown', {})).toBeNull();
  });

  it('resolveWriteInvoke maps save_entry fields', () => {
    expect(
      resolveWriteInvoke('/api/save', {
        layer: 'digest',
        id: 'x/y.md',
        content: '# hi',
      })
    ).toEqual({
      cmd: 'save_entry',
      args: { layer: 'digest', id: 'x/y.md', content: '# hi' },
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
        key: 'doc:20260827/why-cache.md',
        highlight: { text: 'cache', occurrence: 0 },
        ts: '20260827T154000',
      }),
    ).toEqual({
      cmd: 'update_doc_highlights',
      args: {
        key: 'doc:20260827/why-cache.md',
        highlight: { text: 'cache', occurrence: 0 },
        ts: '20260827T154000',
      },
    });
  });

  it('doc-highlights invoke does not route to Annotation write commands', () => {
    const resolved = resolveWriteInvoke('/api/doc-highlights', {
      key: 'doc:task_1',
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
        channel: 'docs',
      }),
    ).toEqual({
      cmd: 'mark_message_channel_read',
      args: { channel: 'docs' },
    });
  });

  it('WRITE_API_INVOKE_MAP 含 show_os_notification', () => {
    expect(WRITE_API_INVOKE_MAP['/api/show-os-notification']?.cmd).toBe(
      'show_os_notification',
    );
  });

  it('resolveWriteInvoke maps show-os-notification title/body/scheme', () => {
    expect(
      resolveWriteInvoke('/api/show-os-notification', {
        title: 'New note',
        body: 'A note was added',
        scheme: 'spark://docs/open?id=a',
      }),
    ).toEqual({
      cmd: 'show_os_notification',
      args: {
        title: 'New note',
        body: 'A note was added',
        scheme: 'spark://docs/open?id=a',
      },
    });
  });

  it('host/api.ts 对外暴露 showOsNotification，传输走 writePost', async () => {
    const api = await import('../../frontend/src/host/api.ts');
    expect(typeof api.showOsNotification).toBe('function');
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({}),
      text: async () => '{}',
    });
    await api.showOsNotification({
      title: 'Read Later',
      body: 'A link was saved',
      scheme: 'spark://read-later/list',
    });
    expect(globalThis.fetch.mock.calls[0][0]).toMatch(/show-os-notification/);
    expect(globalThis.fetch.mock.calls[0][1].method).toBe('POST');
    expect(JSON.parse(globalThis.fetch.mock.calls[0][1].body)).toEqual({
      title: 'Read Later',
      body: 'A link was saved',
      scheme: 'spark://read-later/list',
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
    await api.markMessageChannelRead('docs');
    expect(globalThis.fetch.mock.calls[0][0]).toMatch(/mark-message-channel-read/);
    expect(globalThis.fetch.mock.calls[0][1].method).toBe('POST');
    expect(JSON.parse(globalThis.fetch.mock.calls[0][1].body)).toEqual({
      channel: 'docs',
    });
  });
});
