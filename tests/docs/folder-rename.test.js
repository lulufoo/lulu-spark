// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { renameFolder, refreshNotesIndex } = vi.hoisted(() => ({
  renameFolder: vi.fn(),
  refreshNotesIndex: vi.fn(async () => true),
}));
vi.mock('../../frontend/src/host/api.ts', async (importOriginal) => ({
  ...(await importOriginal()),
  renameFolder,
}));
vi.mock('../../frontend/src/notes/commands/reload-index.ts', () => ({ refreshNotesIndex }));
vi.mock('../../frontend/src/notes/commands/cards.ts', () => ({ loadTitles: vi.fn() }));

import { state } from '../../frontend/src/host/state.ts';
import { cancelFolderRename } from '../../frontend/src/notes/commands/folder-rename.ts';
import { closeFolderMenu } from '../../frontend/src/notes/commands/folder-create.ts';
import { dirKeyOf, dirTreeStore } from '../../frontend/src/notes/state/dir-tree.ts';
import { DirTree } from '../../frontend/src/notes/ui/dir-tree.tsx';
import { FolderMenu, onDirectoryBackgroundMenu } from '../../frontend/src/notes/ui/folder-menu.tsx';

function seed() {
  state.workspaces = {
    current: 'default',
    list: [{ workspace_id: 'default', workspace_name: 'Default', view_mode: 'directory', is_default: true }],
  };
  state.index.folders = [];
  state.index.topicTitles = { ideas: 'Ideas' };
  state.index.topicOrder = ['ideas'];
  state.index.data = {
    n1: { project_id: 'ideas', raw_path: 'plans/q1/a.md', title: '笔记' },
  };
  state.ui.activeTopic = 'ideas';
  state.ui.activeDate = dirKeyOf('ideas/plans');
  dirTreeStore.set({ expanded: ['ideas/plans', 'ideas/plans/q1'] });
  window.location.hash = '#/spark';
}

function setInput(input, value) {
  const proto = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value');
  proto.set.call(input, value);
  input.dispatchEvent(new Event('input', { bubbles: true }));
}

function Tree() {
  return createElement(
    'div',
    { id: 'sidebar-tree-zone', onContextMenu: onDirectoryBackgroundMenu },
    createElement(DirTree),
    createElement(FolderMenu),
  );
}

beforeEach(() => {
  renameFolder.mockReset();
  renameFolder.mockResolvedValue({ ok: true, path: 'archive' });
  refreshNotesIndex.mockClear();
  vi.spyOn(window, 'alert').mockImplementation(() => {});
  cancelFolderRename();
  closeFolderMenu();
  seed();
});

describe('directory rename folder', () => {
  it('renames the folder from the row and keeps the list on the new path', async () => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => root.render(createElement(Tree)));
    const button = host.querySelector('[data-node="ideas/plans"] button');
    await act(async () => {
      button.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 4, clientY: 4 }));
    });
    await act(async () => {
      [...document.querySelectorAll('[data-role="folder-menu"] button')]
        .find((item) => item.textContent === 'Rename')
        .dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    });
    const input = host.querySelector('[aria-label="Folder name"]');
    expect(input.value).toBe('plans');
    await act(async () => {
      setInput(input, 'archive');
      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
      input.blur();
    });
    expect(renameFolder).toHaveBeenCalledTimes(1);
    expect(renameFolder).toHaveBeenCalledWith({
      workspace_id: 'default',
      project_id: 'ideas',
      path: 'plans',
      name: 'archive',
    });
    expect(refreshNotesIndex).toHaveBeenCalledTimes(1);
    expect(state.ui.activeDate).toBe(dirKeyOf('ideas/archive'));
    expect(dirTreeStore.getSnapshot().expanded).toContain('ideas/archive');
    await act(async () => root.unmount());
  });

  it('an empty name cancels, and blank space has no Rename', async () => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => root.render(createElement(Tree)));
    const button = host.querySelector('[data-node="ideas/plans"] button');
    await act(async () => {
      button.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 2, clientY: 2 }));
    });
    await act(async () => {
      [...document.querySelectorAll('[data-role="folder-menu"] button')]
        .find((item) => item.textContent === 'Rename')
        .dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    });
    const input = host.querySelector('[aria-label="Folder name"]');
    await act(async () => {
      setInput(input, '   ');
      input.blur();
    });
    expect(renameFolder).not.toHaveBeenCalled();
    expect(host.querySelector('[aria-label="Folder name"]')).toBeNull();
    const zone = host.querySelector('#sidebar-tree-zone');
    await act(async () => {
      zone.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 8, clientY: 8 }));
    });
    const labels = [...document.querySelectorAll('[data-role="folder-menu"] button')].map((item) => item.textContent);
    expect(labels).toEqual(['New folder']);
    await act(async () => root.unmount());
  });

  it('Enter with a listed folder opens the name editor, including when focus is not on the row', async () => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => root.render(createElement(Tree)));
    await act(async () => {
      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    });
    const input = host.querySelector('[aria-label="Folder name"]');
    expect(input).not.toBeNull();
    expect(input.value).toBe('plans');
    expect(renameFolder).not.toHaveBeenCalled();
    await act(async () => {
      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    });
    const field = document.createElement('input');
    document.body.appendChild(field);
    field.focus();
    await act(async () => {
      field.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    });
    expect(host.querySelector('[aria-label="Folder name"]')).toBeNull();
    field.remove();
    await act(async () => root.unmount());
  });
});
