// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { createNote, refreshNotesIndex } = vi.hoisted(() => ({
  createNote: vi.fn(),
  refreshNotesIndex: vi.fn(async () => true),
}));
vi.mock('../../frontend/src/host/api.ts', async (importOriginal) => ({
  ...(await importOriginal()),
  createNote,
}));
vi.mock('../../frontend/src/notes/commands/reload-index.ts', () => ({ refreshNotesIndex }));

import { state } from '../../frontend/src/host/state.ts';
import { startCreateDocument } from '../../frontend/src/notes/commands/create-document.ts';
import { cancelTimelineCreate } from '../../frontend/src/notes/commands/timeline-create.ts';
import { timelineCreateStore } from '../../frontend/src/notes/state/timeline-create.ts';
import { NotesDocList } from '../../frontend/src/notes/ui/cards.tsx';

function seed() {
  state.workspaces = {
    current: 'default',
    list: [{ workspace_id: 'default', workspace_name: 'Default', view_mode: 'timeline', is_default: true }],
  };
  state.index.topicTitles = { ideas: 'Ideas', inbox: 'Inbox' };
  state.index.topicOrder = ['ideas', 'inbox'];
  state.index.topicDefault = { inbox: true };
  state.ui.activeTopic = '';
  state.ui.activeDate = '20260710';
  state.index.data = {};
}

function setInput(input, value) {
  const proto = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value');
  proto.set.call(input, value);
  input.dispatchEvent(new Event('input', { bubbles: true }));
}

beforeEach(() => {
  createNote.mockReset();
  createNote.mockResolvedValue({ id: 'n1', ok: true });
  refreshNotesIndex.mockClear();
  cancelTimelineCreate();
  seed();
});

describe('timeline title card', () => {
  it('shows a title field on the list, and a blank field cancels without writing', async () => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => {
      root.render(createElement(NotesDocList, {
        entries: [{ id: 'old', entry: { title: 'Existing', project_id: 'ideas', raw_path: 'a.md' } }],
        date: '20260710',
      }));
    });
    await act(async () => {
      startCreateDocument();
    });
    const input = host.querySelector('[aria-label="Document title"]');
    expect(input).not.toBeNull();
    expect(host.querySelector('[data-role="timeline-create"] .doc-topic').textContent).toBe('Inbox');
    expect(host.querySelector('[data-role="timeline-create"]').compareDocumentPosition(host.querySelector('.doc-card[data-id="old"]')) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();

    await act(async () => {
      input.blur();
    });
    expect(host.querySelector('[data-role="timeline-create"]')).toBeNull();
    expect(createNote).not.toHaveBeenCalled();
    await act(async () => root.unmount());
  });

  it('Enter creates an empty document with that title and does not create it again on blur', async () => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => root.render(createElement(NotesDocList, { entries: [], date: '20260710' })));
    await act(async () => {
      startCreateDocument();
    });
    const input = host.querySelector('[aria-label="Document title"]');
    await act(async () => {
      setInput(input, '  Design notes  ');
    });
    await act(async () => {
      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
      input.blur();
    });
    expect(createNote).toHaveBeenCalledTimes(1);
    expect(createNote).toHaveBeenCalledWith({
      title: 'Design notes',
      source_type: 'jot',
      workspace_id: 'default',
      project_id: 'inbox',
      content: '',
    });
    expect(timelineCreateStore.getSnapshot().project).toBe('');
    expect(refreshNotesIndex).toHaveBeenCalledTimes(1);
    await act(async () => root.unmount());
  });

  it('Escape removes the card and blur does not create', async () => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => root.render(createElement(NotesDocList, { entries: [], date: '20260710' })));
    await act(async () => {
      startCreateDocument();
    });
    const input = host.querySelector('[aria-label="Document title"]');
    await act(async () => {
      setInput(input, 'Keep');
      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
      input.blur();
    });
    expect(createNote).not.toHaveBeenCalled();
    expect(host.querySelector('[data-role="timeline-create"]')).toBeNull();
    await act(async () => root.unmount());
  });
});
