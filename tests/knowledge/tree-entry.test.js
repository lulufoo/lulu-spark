import { describe, expect, it } from 'vitest';
import {
  applyCreateInMemory,
  applyDeleteInMemory,
  creatingPathFor,
  insertDraftNode,
  normalizeCreateName,
  parentPathForAdd,
  pathIsInside,
} from '../../frontend/src/knowledge/state/tree-entry.ts';

describe('knowledge tree add/delete paths', () => {
  it('puts add under a directory and beside a file', () => {
    expect(parentPathForAdd('docs', true)).toBe('docs');
    expect(parentPathForAdd('docs/guide.md', false)).toBe('docs');
    expect(parentPathForAdd('readme.md', false)).toBe('');
    expect(parentPathForAdd('', true)).toBe('');
  });

  it('normalizes file names to .md and rejects other suffixes', () => {
    expect(normalizeCreateName('guide', 'file')).toBe('guide.md');
    expect(normalizeCreateName('guide.md', 'file')).toBe('guide.md');
    expect(normalizeCreateName('guide.MD', 'file')).toBe('guide.md');
    expect(normalizeCreateName('guide.txt', 'file')).toBeNull();
    expect(normalizeCreateName('notes', 'dir')).toBe('notes');
    expect(normalizeCreateName('a/b', 'dir')).toBeNull();
  });

  it('inserts a draft then replaces it with the created file', () => {
    const dirCache = new Map();
    const drafted = insertDraftNode([], '', 'file');
    expect(drafted[0].relative_path).toBe(creatingPathFor(''));
    const applied = applyCreateInMemory(drafted, dirCache, '', 'untitled.md', 'file');
    expect(applied.nodes).toHaveLength(1);
    expect(applied.nodes[0]).toMatchObject({
      name: 'untitled.md',
      relative_path: 'untitled.md',
      is_dir: false,
    });
    expect(applied.selectedPath).toBe('untitled.md');
    expect(dirCache.get('')).toEqual([
      { name: 'untitled.md', relative_path: 'untitled.md', is_dir: false },
    ]);
  });

  it('clears opened path when deleting its parent folder', () => {
    const dirCache = new Map([
      ['', [{ name: 'docs', relative_path: 'docs', is_dir: true }]],
      ['docs', [{ name: 'a.md', relative_path: 'docs/a.md', is_dir: false }]],
    ]);
    const applied = applyDeleteInMemory(
      [
        {
          name: 'docs',
          relative_path: 'docs',
          is_dir: true,
          expanded: true,
          loaded: true,
          children: [
            {
              name: 'a.md',
              relative_path: 'docs/a.md',
              is_dir: false,
              expanded: false,
              loaded: false,
              children: [],
            },
          ],
        },
      ],
      dirCache,
      'docs/a.md',
      'docs/a.md',
      'docs',
    );
    expect(applied.nodes).toEqual([]);
    expect(applied.selectedPath).toBe('');
    expect(applied.openedPath).toBe('');
    expect(applied.openedChanged).toBe(true);
    expect(pathIsInside('docs/a.md', 'docs')).toBe(true);
  });
});
