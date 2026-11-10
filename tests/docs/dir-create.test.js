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
vi.mock('../../frontend/src/notes/commands/cards.ts', () => ({ loadTitles: vi.fn() }));

import { state } from '../../frontend/src/host/state.ts';
import { startCreateDocument } from '../../frontend/src/notes/commands/create-document.ts';
import { cancelDirCreate } from '../../frontend/src/notes/commands/dir-create.ts';
import { dirKeyOf } from '../../frontend/src/notes/state/dir-tree.ts';
import { dirCreateStore } from '../../frontend/src/notes/state/dir-create.ts';
import { DirTree } from '../../frontend/src/notes/ui/dir-tree.tsx';

function seed({ topic = 'ideas', listed = dirKeyOf('ideas/plans/q1'), docs = true } = {}) {
  state.workspaces = {
    current: 'default',
    list: [{ workspace_id: 'default', workspace_name: 'Default', view_mode: 'directory', is_default: true }],
  };
  state.index.topicTitles = { ideas: 'Ideas', empty: 'Empty project' };
  state.index.topicOrder = ['ideas', 'empty'];
  state.index.topicDefault = { empty: true };
  state.index.data = docs ? { n1: { project_id: 'ideas', raw_path: 'plans/q1/a.md', title: '笔记' } } : {};
  state.ui.activeTopic = topic;
  state.ui.activeDate = listed;
}

function setInput(input, value) {
  const proto = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value');
  proto.set.call(input, value);
  input.dispatchEvent(new Event('input', { bubbles: true }));
}

beforeEach(() => {
  createNote.mockReset();
  createNote.mockResolvedValue({ id: 'n2', ok: true });
  refreshNotesIndex.mockClear();
  vi.spyOn(window, 'alert').mockImplementation(() => {});
  cancelDirCreate();
  seed();
});

describe('directory file-name row', () => {
  it('asks under the listed folder, and a blank name cancels', async () => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => root.render(createElement(DirTree)));
    await act(async () => {
      startCreateDocument();
    });
    const input = host.querySelector('[aria-label="Markdown file name"]');
    const row = host.querySelector('[data-role="dir-create"]');
    expect(input).not.toBeNull();
    expect(row.compareDocumentPosition(host.querySelector('[data-node="ideas/plans/q1"]')) & Node.DOCUMENT_POSITION_PRECEDING).toBeTruthy();
    await act(async () => {
      input.blur();
    });
    expect(host.querySelector('[data-role="dir-create"]')).toBeNull();
    expect(createNote).not.toHaveBeenCalled();
    await act(async () => root.unmount());
  });

  it('Enter creates an empty file with that name and does not create it again on blur', async () => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => root.render(createElement(DirTree)));
    await act(async () => {
      startCreateDocument();
    });
    const input = host.querySelector('[aria-label="Markdown file name"]');
    await act(async () => {
      setInput(input, 'architecture-notes.md');
      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
      input.blur();
    });
    expect(createNote).toHaveBeenCalledTimes(1);
    expect(createNote).toHaveBeenCalledWith({
      title: 'architecture-notes',
      filename: 'architecture-notes.md',
      folder: 'plans/q1',
      source_type: 'jot',
      workspace_id: 'default',
      project_id: 'ideas',
      content: '',
    });
    expect(dirCreateStore.getSnapshot().project).toBe('');
    expect(refreshNotesIndex).toHaveBeenCalledTimes(1);
    await act(async () => root.unmount());
  });

  it('an empty project shows the name row instead of the empty message', async () => {
    seed({ topic: 'empty', listed: null, docs: false });
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => root.render(createElement(DirTree)));
    expect(host.textContent).toContain('This project has no documents yet.');
    await act(async () => {
      startCreateDocument();
    });
    expect(host.querySelector('[aria-label="Markdown file name"]')).not.toBeNull();
    expect(host.textContent).not.toContain('This project has no documents yet.');
    expect(dirCreateStore.getSnapshot().folder).toBe('/');
    await act(async () => root.unmount());
  });
});
