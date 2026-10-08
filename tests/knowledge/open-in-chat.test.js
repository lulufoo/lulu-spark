// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { openKnowledgeInChat } from '../../frontend/src/knowledge/commands/open-in-chat.ts';
import * as homeOpen from '../../frontend/src/home/commands/open-in-chat.ts';
import * as api from '../../frontend/src/host/api.ts';

describe('openKnowledgeInChat', () => {
  /** @type {import('vitest').MockInstance} */
  let invokeSpy;
  /** @type {import('vitest').MockInstance} */
  let openSpy;

  beforeEach(() => {
    openSpy = vi.spyOn(homeOpen, 'openPathInChat').mockResolvedValue();
    invokeSpy = vi.spyOn(api, 'invoke').mockImplementation(async (cmd, args) => {
      if (cmd === 'remember_knowledge_doc') {
        return { ok: true, id: 'kbdocid12ab' };
      }
      return args;
    });
  });

  afterEach(() => {
    invokeSpy.mockRestore();
    openSpy.mockRestore();
  });

  it('remembers the path then stages knowledge source', async () => {
    await openKnowledgeInChat('/Users/me/knowledge/demo/doc.md');
    expect(invokeSpy).toHaveBeenCalledWith('remember_knowledge_doc', {
      path: '/Users/me/knowledge/demo/doc.md',
    });
    expect(openSpy).toHaveBeenCalledWith('/Users/me/knowledge/demo/doc.md', {
      kind: 'knowledge',
      id: 'kbdocid12ab',
    });
  });

  it('rejects when remember_knowledge_doc returns no id', async () => {
    invokeSpy.mockResolvedValue({ error: 'Missing path' });
    await expect(openKnowledgeInChat('/Users/me/knowledge/demo/doc.md')).rejects.toThrow(
      'Missing source',
    );
    expect(openSpy).not.toHaveBeenCalled();
  });
});
