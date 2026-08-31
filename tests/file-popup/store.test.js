import { describe, expect, it } from 'vitest';
import {
  commonPathFromAbsPath,
  layerFromAbsPath,
  titleFromPath,
} from '../../frontend/src/file-popup/state/store.ts';

describe('file-popup path helpers', () => {
  it('reads layer from the last raw or digest segment', () => {
    expect(layerFromAbsPath('/kb/notes/raw/inbox/a.md')).toBe('raw');
    expect(layerFromAbsPath('/kb/notes/digest/inbox/a.md')).toBe('digest');
    expect(layerFromAbsPath('C:\\kb\\notes\\digest\\inbox\\a.md')).toBe('digest');
    expect(layerFromAbsPath('/tmp/other.md')).toBe('raw');
  });

  it('reads notes common_path after /notes/raw/ or /notes/digest/', () => {
    expect(commonPathFromAbsPath('/kb/notes/raw/inbox/a.md')).toBe('inbox/a.md');
    expect(commonPathFromAbsPath('/kb/notes/digest/topic/b.md')).toBe('topic/b.md');
    expect(commonPathFromAbsPath('C:\\kb\\notes\\raw\\inbox\\a.md')).toBe('inbox/a.md');
    expect(commonPathFromAbsPath('/tmp/other.md')).toBe('');
  });

  it('prefers an explicit title over the filename', () => {
    expect(titleFromPath('/kb/notes/raw/inbox/a.md', 'Given')).toBe('Given');
    expect(titleFromPath('/kb/notes/raw/inbox/a.md')).toBe('a');
  });
});
