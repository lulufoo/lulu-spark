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
    expect(normalizeViewerState({ repo: 'myrepo', path: 'a.md' })).toEqual({ id: '' });
  });

  it('resolveKnowledgeLanding prefers URL id', () => {
    expect(
      resolveKnowledgeLanding('myrepo', 'url-id', ['myrepo'], { id: 'old-id' }, 'myrepo'),
    ).toEqual({ topic: 'myrepo', id: 'url-id' });
  });

  it('resolveKnowledgeLanding restores remembered id when URL has topic only', () => {
    expect(
      resolveKnowledgeLanding('myrepo', '', ['myrepo', 'other'], { id: 'a-id' }, 'myrepo'),
    ).toEqual({ topic: 'myrepo', id: 'a-id' });
  });

  it('resolveKnowledgeLanding clears id when remembered topic differs', () => {
    expect(
      resolveKnowledgeLanding('other', '', ['myrepo', 'other'], { id: 'a-id' }, 'myrepo'),
    ).toEqual({ topic: 'other', id: '' });
  });

  it('resolveKnowledgeLanding uses remembered topic when URL has none', () => {
    expect(
      resolveKnowledgeLanding('', '', ['myrepo', 'other'], { id: 'b-id' }, 'other'),
    ).toEqual({ topic: 'other', id: 'b-id' });
    expect(
      resolveKnowledgeLanding('', '', ['myrepo', 'other'], { id: 'b-id' }, 'gone'),
    ).toEqual({ topic: 'myrepo', id: '' });
  });

  it('knowledgeDocHash encodes topic and optional id', () => {
    expect(knowledgeTopicHash('myrepo')).toBe('#/knowledge/' + encodeURIComponent('myrepo'));
    expect(knowledgeDocHash('myrepo')).toBe('#/knowledge/' + encodeURIComponent('myrepo'));
    expect(knowledgeDocHash('myrepo', 'abc')).toBe(
      '#/knowledge/' + encodeURIComponent('myrepo') + '?id=' + encodeURIComponent('abc'),
    );
  });
});
