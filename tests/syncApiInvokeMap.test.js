import { describe, expect, it } from 'vitest';
import {
  SYNC_API_INVOKE_MAP,
  resolveSyncInvoke,
} from '../frontend/js/syncApiInvokeMap.js';

/** [path, cmd, payload keys forwarded from HTTP body] */
const P4_PATHS = [
  ['/api/commit', 'corpus_git_commit', ['message', 'files']],
  ['/api/pull', 'corpus_git_pull', []],
  ['/api/delete', 'delete_entry', ['id']],
  ['/api/move-project', 'move_entry_project', ['id', 'new_project']],
  ['/api/gh-move', 'gh_move_assets', ['src_url', 'dst_dir_url']],
  ['/api/gh-delete', 'gh_delete_assets', ['url']],
  [
    '/api/settle',
    'settle_entry',
    ['common_path', 'comment_id', 'layer', 'doc_theme', 'slug', 'content'],
  ],
  ['/api/draft', 'save_comment_draft', ['common_path', 'content']],
  ['/api/kb/commit', 'kb_git_commit', ['repo', 'message']],
  ['/api/kb/revert', 'kb_git_revert', ['repo', 'path', 'type']],
  ['/api/open-iterm', 'open_kb_in_iterm', ['repo']],
  ['/api/corpus-revert', 'corpus_git_revert', ['path', 'type']],
];

describe('syncApiInvokeMap', () => {
  it('covers all 10 P4 POST paths with command names', () => {
    expect(Object.keys(SYNC_API_INVOKE_MAP)).toHaveLength(12);
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

  it('commit maps body into payload for corpus_git_commit', () => {
    expect(
      resolveSyncInvoke('/api/commit', { message: 'x', files: ['a.md'] })
    ).toEqual({
      cmd: 'corpus_git_commit',
      args: { payload: { message: 'x', files: ['a.md'] } },
    });
  });

  it('pull maps empty payload for corpus_git_pull', () => {
    expect(resolveSyncInvoke('/api/pull', {})).toEqual({
      cmd: 'corpus_git_pull',
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

  it('gh-move maps body into payload for gh_move_assets', () => {
    expect(
      resolveSyncInvoke('/api/gh-move', {
        src_url: 'https://github.com/o/r/a',
        dst_dir_url: 'https://github.com/o/r/tree/main/b',
      })
    ).toEqual({
      cmd: 'gh_move_assets',
      args: {
        payload: {
          src_url: 'https://github.com/o/r/a',
          dst_dir_url: 'https://github.com/o/r/tree/main/b',
        },
      },
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

  it('kb commit maps body into payload for kb_git_commit', () => {
    expect(
      resolveSyncInvoke('/api/kb/commit', { repo: 'o/r', message: 'm' })
    ).toEqual({
      cmd: 'kb_git_commit',
      args: { payload: { repo: 'o/r', message: 'm' } },
    });
  });

  it('kb revert maps body into payload for kb_git_revert', () => {
    expect(
      resolveSyncInvoke('/api/kb/revert', {
        repo: 'o/r',
        path: 'a.md',
        type: 'file',
      })
    ).toEqual({
      cmd: 'kb_git_revert',
      args: { payload: { repo: 'o/r', path: 'a.md', type: 'file' } },
    });
  });

  it('open-iterm maps body into payload for open_kb_in_iterm', () => {
    expect(resolveSyncInvoke('/api/open-iterm', { repo: 'o/r' })).toEqual({
      cmd: 'open_kb_in_iterm',
      args: { payload: { repo: 'o/r' } },
    });
  });

  it('does not move search reindex commands into the sync API map', () => {
    const syncCommands = Object.values(SYNC_API_INVOKE_MAP).map((entry) => entry.cmd);

    expect(syncCommands).not.toContain('reindex_kb_repo');
    expect(syncCommands).not.toContain('reindex_workbench');
    expect(syncCommands).not.toContain('sync_knowledge_corpus');
  });

  it('resolveSyncInvoke returns null for unknown path', () => {
    expect(resolveSyncInvoke('/api/nope', {})).toBeNull();
  });

  it('corpus-revert maps path+type into payload for corpus_git_revert', () => {
    expect(
      resolveSyncInvoke('/api/corpus-revert', { path: 'raw/foo/bar.md', type: 'modified' })
    ).toEqual({
      cmd: 'corpus_git_revert',
      args: { payload: { path: 'raw/foo/bar.md', type: 'modified' } },
    });
  });

  it('corpus-revert with empty body passes undefined values without crashing', () => {
    const result = resolveSyncInvoke('/api/corpus-revert', {});
    expect(result?.cmd).toBe('corpus_git_revert');
    expect(result?.args).toEqual({ payload: { path: undefined, type: undefined } });
  });
});
