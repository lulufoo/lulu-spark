import { afterEach, describe, expect, it } from 'vitest';
import { stagedIdentityKey } from '../../frontend/src/home/state/identity.ts';
import { state } from '../../frontend/src/host/state.ts';

describe('stagedIdentityKey', () => {
  afterEach(() => {
    state.ui.knowledgeRoot = '';
  });

  it('builds notes key from /notes/raw/ or /notes/digest/', () => {
    expect(stagedIdentityKey('notes', '/kb/notes/raw/inbox/a.md')).toBe('notes:inbox/a.md');
    expect(stagedIdentityKey('notes', '/kb/notes/digest/topic/b.md')).toBe('notes:topic/b.md');
  });

  it('builds knowledge key from knowledgeRoot', () => {
    state.ui.knowledgeRoot = '/Users/me/knowledge';
    expect(stagedIdentityKey('knowledge', '/Users/me/knowledge/demo/doc.md')).toBe(
      'knowledge:demo/doc.md',
    );
    expect(stagedIdentityKey('knowledge', '/Users/me/knowledge/readme.md')).toBe(
      'knowledge:readme.md',
    );
  });

  it('returns no key for bare stage or unknown kind', () => {
    state.ui.knowledgeRoot = '/Users/me/knowledge';
    expect(stagedIdentityKey(undefined, '/Users/me/knowledge/demo/doc.md')).toBeUndefined();
    expect(stagedIdentityKey('', '/tmp/x.md')).toBeUndefined();
    expect(stagedIdentityKey('other', '/tmp/x.md')).toBeUndefined();
  });
});
