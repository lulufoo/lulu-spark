import { describe, expect, it } from 'vitest';
import {
  SYNC_API_INVOKE_MAP,
  resolveSyncInvoke,
} from '../../frontend/src/host/syncApiInvokeMap.ts';

/** [path, cmd, payload keys forwarded from HTTP body] */
const P4_PATHS = [
  ['/api/delete', 'delete_entry', ['id']],
  ['/api/move-project', 'move_entry_project', ['id', 'new_project']],
  ['/api/draft', 'save_comment_draft', ['common_path', 'content']],
];

describe('syncApiInvokeMap', () => {
  it('covers all P4 POST paths with command names', () => {
    expect(Object.keys(SYNC_API_INVOKE_MAP)).toHaveLength(3);
    for (const [path, cmd] of P4_PATHS) {
      expect(SYNC_API_INVOKE_MAP[path]?.cmd, path).toBe(cmd);
    }
  });

  it('every path wraps Tauri invoke args in payload (Rust sync commands)', () => {
    for (const [path] of P4_PATHS) {
      const resolved = resolveSyncInvoke(path, {});
      expect(resolved?.args, path).toEqual({ payload: {} });
      expect(Object.keys(resolved?.args ?? {}), path).toEqual(['payload']);
    }
  });

  it('delete maps body into payload for delete_entry', () => {
    expect(resolveSyncInvoke('/api/delete', { id: 'abc' })).toEqual({
      cmd: 'delete_entry',
      args: { payload: { id: 'abc' } },
    });
  });

  it('move-project maps body into payload for move_entry_project', () => {
    expect(
      resolveSyncInvoke('/api/move-project', {
        id: 'e1',
        new_project: 'foo',
      })
    ).toEqual({
      cmd: 'move_entry_project',
      args: { payload: { id: 'e1', new_project: 'foo' } },
    });
  });

  it('draft maps body into payload for save_comment_draft', () => {
    expect(
      resolveSyncInvoke('/api/draft', {
        common_path: 'ai/x.md',
        content: 'draft',
      })
    ).toEqual({
      cmd: 'save_comment_draft',
      args: { payload: { common_path: 'ai/x.md', content: 'draft' } },
    });
  });

  it('kb git paths are no longer mapped', () => {
    expect(resolveSyncInvoke('/api/kb/commit', { repo: 'o/r', message: 'm' })).toBeNull();
    expect(resolveSyncInvoke('/api/kb/revert', { repo: 'o/r', path: 'a.md', type: 'file' })).toBeNull();
  });

  it('spark git paths are no longer mapped', () => {
    expect(resolveSyncInvoke('/api/commit', { message: 'x' })).toBeNull();
    expect(resolveSyncInvoke('/api/pull', {})).toBeNull();
    expect(resolveSyncInvoke('/api/spark-revert', { path: 'raw/foo.md', type: 'modified' })).toBeNull();
  });

  it('resolveSyncInvoke returns null for unknown path', () => {
    expect(resolveSyncInvoke('/api/nope', {})).toBeNull();
  });

});
