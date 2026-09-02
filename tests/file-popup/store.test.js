import { describe, expect, it } from 'vitest';
import { emptyFilePopupView, titleFromPath } from '../../frontend/src/file-popup/state/store.ts';

describe('file-popup path helpers', () => {
  it('prefers an explicit title over the filename', () => {
    expect(titleFromPath('/kb/notes/raw/inbox/a.md', 'Given')).toBe('Given');
    expect(titleFromPath('/kb/notes/raw/inbox/a.md')).toBe('a');
  });

  it('empty view has no layer and no identity key', () => {
    const empty = emptyFilePopupView();
    expect(empty.identityKey).toBe('');
    expect(empty).not.toHaveProperty('layer');
  });
});
