// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  copyKnowledgeReference,
  knowledgeReferenceDraft,
  openKnowledgeInChat,
} from '../../frontend/src/knowledge/commands/open-in-chat.ts';
import * as homeOpen from '../../frontend/src/home/commands/open-in-chat.ts';
import * as api from '../../frontend/src/host/api.ts';
import { parseReferences } from '../../frontend/src/home/references/index.ts';
import { state } from '../../frontend/src/host/state.ts';

const ID = 'b5c48a6383d20ccb1251d2cc0feb9e3a';

function resetViewer() {
  state.viewer.isKb = false;
  state.viewer.kbId = null;
  state.viewer.rawText = '';
}

describe('knowledgeReferenceDraft', () => {
  it('uses the document H1 when the open body has one', () => {
    expect(knowledgeReferenceDraft(ID, '# Design Notes\n\nbody')).toBe(
      `[Design Notes](knowledge:${ID}) `,
    );
  });

  it('uses the title hint when there is no H1', () => {
    expect(knowledgeReferenceDraft(ID, '', 'design-notes')).toBe(`[design-notes](knowledge:${ID}) `);
  });

  it('falls back to knowledge when there is no H1 or hint', () => {
    expect(knowledgeReferenceDraft('ab12')).toBe('[knowledge](knowledge:ab12) ');
  });

  it('is a reference the composer shows as a chip', () => {
    const [segment] = parseReferences(knowledgeReferenceDraft(ID, '# Design Notes'));
    expect(segment).toMatchObject({ type: 'ref', kind: 'knowledge', id: ID, title: 'Design Notes' });
  });
});

describe('openKnowledgeInChat', () => {
  /** @type {import('vitest').MockInstance} */
  let invokeSpy;
  /** @type {import('vitest').MockInstance} */
  let draftSpy;

  beforeEach(() => {
    resetViewer();
    draftSpy = vi.spyOn(homeOpen, 'openDraftInChat').mockResolvedValue();
    invokeSpy = vi.spyOn(api, 'invoke').mockResolvedValue({});
  });

  afterEach(() => {
    invokeSpy.mockRestore();
    draftSpy.mockRestore();
    resetViewer();
  });

  it('opens a new chat prefilled with the document reference', async () => {
    await openKnowledgeInChat(ID);
    expect(invokeSpy).not.toHaveBeenCalled();
    expect(draftSpy).toHaveBeenCalledWith(`[knowledge](knowledge:${ID}) `);
  });

  it('prefills the open viewer H1 when the current knowledge document has one', async () => {
    state.viewer.isKb = true;
    state.viewer.kbId = ID;
    state.viewer.rawText = '# Design Notes\n\nbody';
    await openKnowledgeInChat(ID);
    expect(draftSpy).toHaveBeenCalledWith(`[Design Notes](knowledge:${ID}) `);
  });

  it('does not stage anything', async () => {
    await openKnowledgeInChat(ID);
    expect(invokeSpy.mock.calls.map(([cmd]) => cmd)).not.toContain('stage_chat_document');
  });

  it('rejects an empty id', async () => {
    await expect(openKnowledgeInChat('  ')).rejects.toThrow('Missing source');
    expect(draftSpy).not.toHaveBeenCalled();
  });

  it('re-enables the button after opening, and after a failure', async () => {
    const button = document.createElement('button');
    await openKnowledgeInChat(ID, button);
    expect(button.disabled).toBe(false);

    draftSpy.mockRejectedValueOnce(new Error('boom'));
    await expect(openKnowledgeInChat(ID, button)).rejects.toThrow('boom');
    expect(button.disabled).toBe(false);
  });
});

describe('copyKnowledgeReference', () => {
  /** @type {import('vitest').Mock} */
  let writeText;

  beforeEach(() => {
    resetViewer();
    writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal('navigator', { clipboard: { writeText } });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    resetViewer();
  });

  it('copies the kind, title and id with no file path', async () => {
    await expect(copyKnowledgeReference(ID)).resolves.toBe(true);
    expect(writeText).toHaveBeenCalledWith(
      `Lulu Spark knowledge\ntitle: knowledge\nid: ${ID}`,
    );
    expect(writeText.mock.calls[0][0]).not.toContain('/Users/');
  });

  it('uses the open viewer H1 as the title', async () => {
    state.viewer.isKb = true;
    state.viewer.kbId = ID;
    state.viewer.rawText = '# Design Notes\n\nbody';
    await copyKnowledgeReference(ID);
    expect(writeText).toHaveBeenCalledWith(
      `Lulu Spark knowledge\ntitle: Design Notes\nid: ${ID}`,
    );
  });

  it('rejects an empty id and copies nothing', async () => {
    await expect(copyKnowledgeReference('  ')).rejects.toThrow('Missing source');
    expect(writeText).not.toHaveBeenCalled();
  });
});
