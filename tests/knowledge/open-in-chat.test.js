// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  knowledgeReferenceDraft,
  openKnowledgeInChat,
} from '../../frontend/src/knowledge/commands/open-in-chat.ts';
import * as homeOpen from '../../frontend/src/home/commands/open-in-chat.ts';
import * as api from '../../frontend/src/host/api.ts';
import { parseReferences } from '../../frontend/src/home/references/index.ts';

const PATH = '/Users/me/knowledge/demo/design-notes.md';
const ID = 'b5c48a6383d20ccb1251d2cc0feb9e3a';

describe('knowledgeReferenceDraft', () => {
  it('links the title to the knowledge id, with a space to keep typing after the chip', () => {
    expect(knowledgeReferenceDraft(PATH, ID)).toBe(`[design-notes](knowledge:${ID}) `);
  });

  it('drops only the last extension and any directory', () => {
    expect(knowledgeReferenceDraft('/a/b/spec.v2.txt', 'ab12')).toBe('[spec.v2](knowledge:ab12) ');
  });

  it('keeps the link on one line and free of brackets', () => {
    expect(knowledgeReferenceDraft('/a/two\nlines [x].md', 'ab12')).toBe(
      '[two lines x](knowledge:ab12) ',
    );
  });

  it('is a reference the composer shows as a chip', () => {
    const [segment] = parseReferences(knowledgeReferenceDraft(PATH, ID));
    expect(segment).toMatchObject({ type: 'ref', kind: 'knowledge', id: ID, title: 'design-notes' });
  });
});

describe('openKnowledgeInChat', () => {
  /** @type {import('vitest').MockInstance} */
  let invokeSpy;
  /** @type {import('vitest').MockInstance} */
  let draftSpy;

  beforeEach(() => {
    draftSpy = vi.spyOn(homeOpen, 'openDraftInChat').mockResolvedValue();
    invokeSpy = vi.spyOn(api, 'invoke').mockImplementation(async (cmd, args) => {
      if (cmd === 'remember_knowledge_doc') {
        return { ok: true, id: ID };
      }
      return args;
    });
  });

  afterEach(() => {
    invokeSpy.mockRestore();
    draftSpy.mockRestore();
  });

  it('remembers the path, then opens a new chat prefilled with the document reference', async () => {
    await openKnowledgeInChat(PATH);
    expect(invokeSpy).toHaveBeenCalledWith('remember_knowledge_doc', { path: PATH });
    expect(draftSpy).toHaveBeenCalledWith(`[design-notes](knowledge:${ID}) `);
  });

  it('does not stage anything', async () => {
    await openKnowledgeInChat(PATH);
    expect(invokeSpy.mock.calls.map(([cmd]) => cmd)).not.toContain('stage_chat_document');
  });

  it('rejects when remember_knowledge_doc returns no id', async () => {
    invokeSpy.mockResolvedValue({ error: 'Missing path' });
    await expect(openKnowledgeInChat(PATH)).rejects.toThrow('Missing source');
    expect(draftSpy).not.toHaveBeenCalled();
  });

  it('rejects an empty path before invoke', async () => {
    await expect(openKnowledgeInChat('  ')).rejects.toThrow('Missing file path');
    expect(invokeSpy).not.toHaveBeenCalled();
  });

  it('re-enables the button after opening, and after a failure', async () => {
    const button = document.createElement('button');
    await openKnowledgeInChat(PATH, button);
    expect(button.disabled).toBe(false);

    draftSpy.mockRejectedValueOnce(new Error('boom'));
    await expect(openKnowledgeInChat(PATH, button)).rejects.toThrow('boom');
    expect(button.disabled).toBe(false);
  });
});
