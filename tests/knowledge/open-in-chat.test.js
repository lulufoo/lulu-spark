// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  knowledgeReferenceDraft,
  openKnowledgeInChat,
} from '../../frontend/src/knowledge/commands/open-in-chat.ts';
import * as homeOpen from '../../frontend/src/home/commands/open-in-chat.ts';
import * as api from '../../frontend/src/host/api.ts';

const PATH = '/Users/me/knowledge/demo/design-notes.md';

describe('knowledgeReferenceDraft', () => {
  it('names the document by title and id', () => {
    expect(knowledgeReferenceDraft(PATH, 'kbdocid12ab')).toBe(
      '请阅读知识库文档「design-notes」（id: kbdocid12ab）',
    );
  });

  it('drops only the last extension and any directory', () => {
    expect(knowledgeReferenceDraft('/a/b/spec.v2.txt', 'x1')).toBe(
      '请阅读知识库文档「spec.v2」（id: x1）',
    );
  });

  it('keeps the draft on one line', () => {
    expect(knowledgeReferenceDraft('/a/two\nlines.md', 'x1')).toBe(
      '请阅读知识库文档「two lines」（id: x1）',
    );
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
        return { ok: true, id: 'kbdocid12ab' };
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
    expect(draftSpy).toHaveBeenCalledWith('请阅读知识库文档「design-notes」（id: kbdocid12ab）');
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
