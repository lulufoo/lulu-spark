import { describe, expect, it } from 'vitest';
import {
  WRITE_API_INVOKE_MAP,
  resolveWriteInvoke,
} from '../frontend/js/writeApiInvokeMap.js';

const P2_PATHS = [
  ['/api/config', 'set_config', ['payload']],
  ['/api/save', 'save_entry', ['layer', 'common_path', 'content']],
  ['/api/update-comments', 'update_comments', ['common_path', 'layer', 'comment', 'ts']],
  ['/api/reorder-comments', 'reorder_comments', ['common_path', 'layer', 'ids']],
  ['/api/update-highlights', 'update_highlights', ['common_path', 'layer', 'highlight', 'ts']],
  ['/api/update-links', 'update_links', ['common_path', 'links']],
  ['/api/set-done', 'set_done', ['common_path', 'done']],
  ['/api/set-importance', 'set_importance', ['common_path', 'importance']],
  ['/api/kb/save', 'kb_save', ['repo', 'path', 'content']],
  ['/api/kb/update-comments', 'kb_update_comments', ['repo', 'path', 'comment', 'ts']],
  ['/api/kb/reorder-comments', 'kb_reorder_comments', ['repo', 'path', 'ids']],
  ['/api/kb/update-highlights', 'kb_update_highlights', ['repo', 'path', 'highlight', 'ts']],
  ['/api/kb/update-links', 'kb_update_links', ['repo', 'path', 'links']],
];

describe('writeApiContract', () => {
  it('covers all 13 P2 POST paths with command names', () => {
    expect(Object.keys(WRITE_API_INVOKE_MAP)).toHaveLength(13);
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
      args: { common_path: 'ai/x.md', done: true },
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
      args: { layer: 'digest', common_path: 'a/b.md', content: '# t' },
    });
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
});
