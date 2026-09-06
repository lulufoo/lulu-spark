// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import {
  applyRenameInMemory,
  isValidEntryName,
  nextRelativePath,
  resolveRenamePathFromKeydown,
  rewriteRelativePath,
  rewriteTreeAfterRename,
} from '../../frontend/src/knowledge/state/tree-rename.ts';

describe('knowledge tree rename paths', () => {
  it('accepts a single path segment and rejects traversal', () => {
    expect(isValidEntryName('guide.md')).toBe(true);
    expect(isValidEntryName('docs')).toBe(true);
    expect(isValidEntryName('')).toBe(false);
    expect(isValidEntryName('..')).toBe(false);
    expect(isValidEntryName('a/b.md')).toBe(false);
    expect(isValidEntryName('a..md')).toBe(false);
  });

  it('builds a same-directory destination path', () => {
    expect(nextRelativePath('readme.md', 'hello.md')).toBe('hello.md');
    expect(nextRelativePath('docs/guide.md', 'intro.md')).toBe('docs/intro.md');
    expect(nextRelativePath('docs/guide.md', 'nested/x.md')).toBeNull();
  });

  it('rewrites the renamed node and its descendants', () => {
    expect(rewriteRelativePath('docs/guide.md', 'docs', 'guides')).toBe('guides/guide.md');
    expect(rewriteRelativePath('readme.md', 'docs', 'guides')).toBe('readme.md');

    const nodes = rewriteTreeAfterRename(
      [
        {
          name: 'docs',
          relative_path: 'docs',
          is_dir: true,
          expanded: true,
          loaded: true,
          children: [
            {
              name: 'guide.md',
              relative_path: 'docs/guide.md',
              is_dir: false,
              expanded: false,
              loaded: false,
              children: [],
            },
          ],
        },
      ],
      'docs',
      'guides',
    );
    expect(nodes[0]).toMatchObject({ name: 'guides', relative_path: 'guides' });
    expect(nodes[0].children[0].relative_path).toBe('guides/guide.md');
  });

  it('rewrites opened and selected paths plus the dir cache', () => {
    const dirCache = new Map([
      ['', [{ name: 'docs', relative_path: 'docs', is_dir: true }]],
      ['docs', [{ name: 'guide.md', relative_path: 'docs/guide.md', is_dir: false }]],
    ]);
    const applied = applyRenameInMemory(
      [
        {
          name: 'docs',
          relative_path: 'docs',
          is_dir: true,
          expanded: true,
          loaded: true,
          children: [],
        },
      ],
      dirCache,
      'docs',
      'docs/guide.md',
      'docs',
      'guides',
    );
    expect(applied.selectedPath).toBe('guides');
    expect(applied.openedPath).toBe('guides/guide.md');
    expect(applied.openedChanged).toBe(true);
    expect(dirCache.has('guides')).toBe(true);
    expect(dirCache.get('guides')[0].relative_path).toBe('guides/guide.md');
  });

  it('Enter after a tree click uses the selected path even if focus left the label', () => {
    const host = document.createElement('div');
    const pane = document.createElement('div');
    host.appendChild(pane);
    expect(
      resolveRenamePathFromKeydown({
        key: 'Enter',
        target: pane,
        host,
        renamingPath: '',
        selectedPath: 'readme.md',
      }),
    ).toBe('readme.md');
  });

  it('Enter in a text field does not start rename', () => {
    const host = document.createElement('div');
    const area = document.createElement('textarea');
    host.appendChild(area);
    expect(
      resolveRenamePathFromKeydown({
        key: 'Enter',
        target: area,
        host,
        renamingPath: '',
        selectedPath: 'readme.md',
      }),
    ).toBe('');
  });
});
