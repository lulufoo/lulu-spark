import { describe, expect, it } from 'vitest';
import {
  SYNC_API_INVOKE_MAP,
  resolveSyncInvoke,
} from '../../frontend/src/host/syncApiInvokeMap.ts';

/** [path, cmd, payload keys forwarded from HTTP body] */
const P4_PATHS = [
  ['/api/commit', 'spark_git_commit', ['message', 'files']],
  ['/api/pull', 'spark_git_pull', []],
  ['/api/delete', 'delete_entry', ['id']],
  ['/api/move-project', 'move_entry_project', ['id', 'new_project']],
  ['/api/gh-delete', 'gh_delete_assets', ['url']],
  [
    '/api/settle',
    'settle_entry',
    ['common_path', 'comment_id', 'layer', 'doc_theme', 'slug', 'content'],
  ],
  ['/api/draft', 'save_comment_draft', ['common_path', 'content']],
  ['/api/spark-revert', 'spark_git_revert', ['path', 'type']],
];

describe('syncApiInvokeMap', () => {
  it('covers all P4 POST paths with command names', () => {
    expect(Object.keys(SYNC_API_INVOKE_MAP)).toHaveLength(8);
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

  it('commit maps body into payload for spark_git_commit', () => {
    expect(
      resolveSyncInvoke('/api/commit', { message: 'x', files: ['a.md'] })
    ).toEqual({
      cmd: 'spark_git_commit',
      args: { payload: { message: 'x', files: ['a.md'] } },
    });
  });

  it('pull maps empty payload for spark_git_pull', () => {
    expect(resolveSyncInvoke('/api/pull', {})).toEqual({
      cmd: 'spark_git_pull',
      args: { payload: {} },
    });
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

  it('gh-delete maps body into payload for gh_delete_assets', () => {
    expect(
      resolveSyncInvoke('/api/gh-delete', {
        url: 'https://github.com/o/r/blob/main/a.md',
      })
    ).toEqual({
      cmd: 'gh_delete_assets',
      args: {
        payload: {
          url: 'https://github.com/o/r/blob/main/a.md',
        },
      },
    });
  });

  it('settle maps body into payload for settle_entry', () => {
    const body = {
      common_path: 'ai/x.md',
      comment_id: 'c1',
      layer: 'raw',
      doc_theme: 't',
      slug: 's',
      content: '# hi',
      repo: 'owner/kb',
    };
    expect(resolveSyncInvoke('/api/settle', body)).toEqual({
      cmd: 'settle_entry',
      args: { payload: body },
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

  it('resolveSyncInvoke returns null for unknown path', () => {
    expect(resolveSyncInvoke('/api/nope', {})).toBeNull();
  });

  it('spark-revert maps path+type into payload for spark_git_revert', () => {
    expect(
      resolveSyncInvoke('/api/spark-revert', { path: 'raw/foo/bar.md', type: 'modified' })
    ).toEqual({
      cmd: 'spark_git_revert',
      args: { payload: { path: 'raw/foo/bar.md', type: 'modified' } },
    });
  });

  it('spark-revert with empty body passes undefined values without crashing', () => {
    const result = resolveSyncInvoke('/api/spark-revert', {});
    expect(result?.cmd).toBe('spark_git_revert');
    expect(result?.args).toEqual({ payload: { path: undefined, type: undefined } });
  });
});
