// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { renameDoc, renameFolder, refreshNotesIndex } = vi.hoisted(() => ({
  renameDoc: vi.fn(),
  renameFolder: vi.fn(),
  refreshNotesIndex: vi.fn(async () => true),
}));
vi.mock('../../frontend/src/host/api.ts', async (importOriginal) => ({
  ...(await importOriginal()),
  renameDoc,
  renameFolder,
}));
vi.mock('../../frontend/src/notes/commands/reload-index.ts', () => ({ refreshNotesIndex }));
vi.mock('../../frontend/src/notes/commands/cards.ts', () => ({ loadTitles: vi.fn() }));

import { state } from '../../frontend/src/host/state.ts';
import { cancelFileRename } from '../../frontend/src/notes/commands/file-rename.ts';
import { cancelFolderRename } from '../../frontend/src/notes/commands/folder-rename.ts';
import { closeFolderMenu } from '../../frontend/src/notes/commands/folder-create.ts';
import { dirKeyOf, dirTreeStore } from '../../frontend/src/notes/state/dir-tree.ts';
import { DirTree } from '../../frontend/src/notes/ui/dir-tree.tsx';
import { FolderMenu } from '../../frontend/src/notes/ui/folder-menu.tsx';

function seed() {
  state.workspaces = {
    current: 'default',
    list: [{ workspace_id: 'default', workspace_name: 'Default', view_mode: 'directory', is_default: true }],
  };
  state.index.folders = [];
  state.index.topicTitles = { ideas: 'Ideas' };
  state.index.topicOrder = ['ideas'];
  state.index.data = {
    n1: { id: 'n1', _id: 'n1', project_id: 'ideas', raw_path: 'plans/q1/a.md', title: '笔记' },
  };
  state.ui.activeTopic = 'ideas';
  state.ui.activeDate = dirKeyOf('ideas/plans');
  state.viewer.outletMode = 'open';
  state.viewer.entry = state.index.data.n1;
  state.viewer.editing = false;
  state.viewer.createSession = null;
  dirTreeStore.set({ expanded: ['ideas/plans', 'ideas/plans/q1'] });
  window.location.hash = '#/spark';
}

function setInput(input, value) {
  const proto = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value');
  proto.set.call(input, value);
  input.dispatchEvent(new Event('input', { bubbles: true }));
}

function Tree() {
  return createElement('div', { id: 'sidebar-tree-zone' }, createElement(DirTree), createElement(FolderMenu));
}

beforeEach(() => {
  renameDoc.mockReset();
  renameDoc.mockResolvedValue({ ok: true, id: 'n1', raw_path: 'plans/q1/archive.md' });
  renameFolder.mockReset();
  refreshNotesIndex.mockReset();
  refreshNotesIndex.mockImplementation(async () => {
    state.index.data.n1 = { ...state.index.data.n1, raw_path: 'plans/q1/archive.md' };
    return true;
  });
  vi.spyOn(window, 'alert').mockImplementation(() => {});
  cancelFileRename();
  cancelFolderRename();
  closeFolderMenu();
  seed();
});

describe('directory rename file', () => {
  it('Enter on an open document edits its file name and does not rename the folder', async () => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => root.render(createElement(Tree)));
    await act(async () => {
      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    });
    const input = host.querySelector('[aria-label="Markdown file name"]');
    expect(input).not.toBeNull();
    expect(input.value).toBe('a.md');
    expect(host.querySelector('[aria-label="Folder name"]')).toBeNull();
    expect(renameFolder).not.toHaveBeenCalled();
    await act(async () => {
      setInput(input, 'archive.md');
      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
      input.blur();
    });
    expect(renameDoc).toHaveBeenCalledTimes(1);
    expect(renameDoc).toHaveBeenCalledWith({
      workspace_id: 'default',
      project_id: 'ideas',
      id: 'n1',
      name: 'archive.md',
    });
    expect(state.viewer.outletMode).toBe('open');
    expect(state.viewer.entry.raw_path).toBe('plans/q1/archive.md');
    expect(state.viewer.entry.title).toBe('笔记');
    await act(async () => root.unmount());
  });

  it('an empty name cancels, and a listed folder still renames when no file is open', async () => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => root.render(createElement(Tree)));
    await act(async () => {
      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    });
    const input = host.querySelector('[aria-label="Markdown file name"]');
    await act(async () => {
      setInput(input, '   ');
      input.blur();
    });
    expect(renameDoc).not.toHaveBeenCalled();
    expect(host.querySelector('[aria-label="Markdown file name"]')).toBeNull();

    state.viewer.outletMode = '';
    state.viewer.entry = null;
    await act(async () => {
      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    });
    const folder = host.querySelector('[aria-label="Folder name"]');
    expect(folder).not.toBeNull();
    expect(folder.value).toBe('plans');
    expect(renameDoc).not.toHaveBeenCalled();
    await act(async () => root.unmount());
  });
});
