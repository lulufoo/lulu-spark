import { describe, expect, it } from 'vitest';
import {
  WRITE_API_INVOKE_MAP,
  resolveWriteInvoke,
} from '../frontend/js/writeApiInvokeMap.js';

const P2_WRITE_PATHS = [
  '/api/config',
  '/api/save',
  '/api/update-comments',
  '/api/reorder-comments',
  '/api/update-highlights',
  '/api/update-links',
  '/api/set-done',
  '/api/set-importance',
  '/api/kb/save',
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
];

describe('writeApiInvokeMap', () => {
  it('covers all 22 P2 POST paths', () => {
    for (const p of P2_WRITE_PATHS) {
      expect(WRITE_API_INVOKE_MAP[p]?.cmd, p).toBeTruthy();
    }
    expect(Object.keys(WRITE_API_INVOKE_MAP)).toHaveLength(22);
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
});
