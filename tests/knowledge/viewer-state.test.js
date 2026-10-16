import { describe, expect, it } from 'vitest';
import {
  isKnowledgeMdPath,
  knowledgeDocHash,
  knowledgeTopicHash,
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

  it('normalizeViewerState keeps id', () => {
    expect(normalizeViewerState({ id: ' abc ' })).toEqual({ id: 'abc' });
    expect(normalizeViewerState({ repo: 'o/r', path: 'a.md' })).toEqual({ id: '' });
  });

  it('resolveKnowledgeLanding prefers URL id', () => {
    expect(
      resolveKnowledgeLanding('o/r', 'url-id', ['o/r'], { id: 'old-id' }, 'o/r'),
    ).toEqual({ topic: 'o/r', id: 'url-id' });
  });

  it('resolveKnowledgeLanding restores remembered id when URL has topic only', () => {
    expect(
      resolveKnowledgeLanding('o/r', '', ['o/r', 'o/x'], { id: 'a-id' }, 'o/r'),
    ).toEqual({ topic: 'o/r', id: 'a-id' });
  });

  it('resolveKnowledgeLanding clears id when remembered topic differs', () => {
    expect(
      resolveKnowledgeLanding('o/x', '', ['o/r', 'o/x'], { id: 'a-id' }, 'o/r'),
    ).toEqual({ topic: 'o/x', id: '' });
  });

  it('resolveKnowledgeLanding uses remembered topic when URL has none', () => {
    expect(
      resolveKnowledgeLanding('', '', ['o/r', 'o/x'], { id: 'b-id' }, 'o/x'),
    ).toEqual({ topic: 'o/x', id: 'b-id' });
    expect(
      resolveKnowledgeLanding('', '', ['o/r', 'o/x'], { id: 'b-id' }, 'gone/r'),
    ).toEqual({ topic: 'o/r', id: '' });
  });

  it('knowledgeDocHash encodes topic and optional id', () => {
    expect(knowledgeTopicHash('o/r')).toBe('#/knowledge/' + encodeURIComponent('o/r'));
    expect(knowledgeDocHash('o/r')).toBe('#/knowledge/' + encodeURIComponent('o/r'));
    expect(knowledgeDocHash('o/r', 'abc')).toBe(
      '#/knowledge/' + encodeURIComponent('o/r') + '?id=' + encodeURIComponent('abc'),
    );
  });
});
