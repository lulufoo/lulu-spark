import { describe, expect, it } from 'vitest';
import {
  isKnowledgeMdPath,
  knowledgeDocHash,
  normalizeViewerState,
  resolveKnowledgeLanding,
} from '../../frontend/src/knowledge/state/viewer-state.ts';

describe('knowledge viewer-state', () => {
  it('accepts only markdown paths', () => {
    expect(isKnowledgeMdPath('docs/a.md')).toBe(true);
    expect(isKnowledgeMdPath('docs/A.MD')).toBe(true);
    expect(isKnowledgeMdPath('docs/a.txt')).toBe(false);
    expect(isKnowledgeMdPath('docs')).toBe(false);
  });

  it('normalizeViewerState drops non-md path', () => {
    expect(normalizeViewerState({ repo: ' o/r ', path: ' notes.txt ' })).toEqual({
      repo: 'o/r',
      path: '',
    });
    expect(normalizeViewerState({ repo: 'o/r', path: 'a.md' })).toEqual({
      repo: 'o/r',
      path: 'a.md',
    });
  });

  it('resolveKnowledgeLanding prefers URL path', () => {
    expect(
      resolveKnowledgeLanding('o/r', 'url.md', ['o/r'], { repo: 'o/r', path: 'old.md' }),
    ).toEqual({ repo: 'o/r', path: 'url.md' });
  });

  it('resolveKnowledgeLanding restores remembered md when URL has repo only', () => {
    expect(
      resolveKnowledgeLanding('o/r', '', ['o/r', 'o/x'], { repo: 'o/r', path: 'a.md' }),
    ).toEqual({ repo: 'o/r', path: 'a.md' });
  });

  it('resolveKnowledgeLanding clears path when remembered repo differs', () => {
    expect(
      resolveKnowledgeLanding('o/x', '', ['o/r', 'o/x'], { repo: 'o/r', path: 'a.md' }),
    ).toEqual({ repo: 'o/x', path: '' });
  });

  it('resolveKnowledgeLanding uses remembered repo when URL has none', () => {
    expect(
      resolveKnowledgeLanding('', '', ['o/r', 'o/x'], { repo: 'o/x', path: 'b.md' }),
    ).toEqual({ repo: 'o/x', path: 'b.md' });
    expect(
      resolveKnowledgeLanding('', '', ['o/r', 'o/x'], { repo: 'gone/r', path: 'b.md' }),
    ).toEqual({ repo: 'o/r', path: '' });
  });

  it('knowledgeDocHash encodes repo and optional path', () => {
    expect(knowledgeDocHash('o/r')).toBe('#/knowledge/' + encodeURIComponent('o/r'));
    expect(knowledgeDocHash('o/r', 'a.md')).toBe(
      '#/knowledge/' + encodeURIComponent('o/r') + '?path=' + encodeURIComponent('a.md'),
    );
  });
});
