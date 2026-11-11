// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { createFolder, deleteFolder, refreshNotesIndex } = vi.hoisted(() => ({
  createFolder: vi.fn(),
  deleteFolder: vi.fn(),
  refreshNotesIndex: vi.fn(async () => true),
}));
vi.mock('../../frontend/src/host/api.ts', async (importOriginal) => ({
  ...(await importOriginal()),
  createFolder,
  deleteFolder,
}));
vi.mock('../../frontend/src/notes/commands/reload-index.ts', () => ({ refreshNotesIndex }));
vi.mock('../../frontend/src/notes/commands/cards.ts', () => ({ loadTitles: vi.fn() }));

import { state } from '../../frontend/src/host/state.ts';
import { cancelFolderCreate } from '../../frontend/src/notes/commands/folder-create.ts';
import { dirKeyOf, dirTreeStore, pathKeys } from '../../frontend/src/notes/state/dir-tree.ts';
import { DirTree } from '../../frontend/src/notes/ui/dir-tree.tsx';
import { FolderMenu, onDirectoryBackgroundClick, onDirectoryBackgroundMenu } from '../../frontend/src/notes/ui/folder-menu.tsx';

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
    n2: { project_id: 'ideas', raw_path: 'a/b/c/deep.md', title: '深' },
  };
  state.ui.activeTopic = 'ideas';
  state.ui.activeDate = dirKeyOf('ideas/plans');
  dirTreeStore.set({ expanded: ['ideas/plans', 'ideas/plans/q1', ...pathKeys('ideas/a/b/c')] });
}

function setInput(input, value) {
  const proto = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value');
  proto.set.call(input, value);
  input.dispatchEvent(new Event('input', { bubbles: true }));
}

function Tree() {
  return createElement(
    'div',
    { id: 'sidebar-tree-zone', onClick: onDirectoryBackgroundClick, onContextMenu: onDirectoryBackgroundMenu },
    createElement(DirTree),
    createElement(FolderMenu),
  );
}

beforeEach(() => {
  createFolder.mockReset();
  createFolder.mockResolvedValue({ ok: true, path: 'plans/design' });
  deleteFolder.mockReset();
  deleteFolder.mockResolvedValue({ ok: true });
  refreshNotesIndex.mockClear();
  vi.spyOn(window, 'alert').mockImplementation(() => {});
  cancelFolderCreate();
  seed();
});

describe('directory new folder', () => {
  it('right-click on a folder asks for a child name and does not keep the name selected', async () => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => root.render(createElement(Tree)));
    const button = host.querySelector('[data-node="ideas/plans"] button');
    const range = document.createRange();
    range.selectNodeContents(button);
    const selection = window.getSelection();
    selection.removeAllRanges();
    selection.addRange(range);
    await act(async () => {
      button.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 12, clientY: 24 }));
    });
    expect(selection.toString()).toBe('');
    expect(state.ui.activeDate).toBe(dirKeyOf('ideas/plans'));
    const item = document.querySelector('[data-role="folder-menu"] button');
    expect(item.textContent).toBe('New folder');
    await act(async () => {
      item.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    });
    const input = host.querySelector('[aria-label="Folder name"]');
    expect(input).not.toBeNull();
    expect(host.querySelector('[data-node="ideas/plans"]').compareDocumentPosition(host.querySelector('[data-role="folder-create"]')) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    await act(async () => {
      input.blur();
    });
    expect(createFolder).not.toHaveBeenCalled();
    expect(host.querySelector('[data-role="folder-create"]')).toBeNull();
    await act(async () => root.unmount());
  });

  it('Enter creates the child folder once', async () => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => root.render(createElement(Tree)));
    const button = host.querySelector('[data-node="ideas/plans"] button');
    await act(async () => {
      button.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 4, clientY: 4 }));
    });
    await act(async () => {
      document.querySelector('[data-role="folder-menu"] button').dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    });
    const input = host.querySelector('[aria-label="Folder name"]');
    await act(async () => {
      setInput(input, 'design');
      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
      input.blur();
    });
    expect(createFolder).toHaveBeenCalledTimes(1);
    expect(createFolder).toHaveBeenCalledWith({
      workspace_id: 'default',
      project_id: 'ideas',
      name: 'design',
      parent: 'plans',
    });
    expect(refreshNotesIndex).toHaveBeenCalledTimes(1);
    await act(async () => root.unmount());
  });

  it('right-click on blank space creates a project-root folder, and a click there clears the listed folder', async () => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => root.render(createElement(Tree)));
    const zone = host.querySelector('#sidebar-tree-zone');
    await act(async () => {
      zone.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 8, clientY: 8 }));
    });
    await act(async () => {
      document.querySelector('[data-role="folder-menu"] button').dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    });
    const input = host.querySelector('[aria-label="Folder name"]');
    await act(async () => {
      setInput(input, 'saved');
      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    });
    expect(createFolder).toHaveBeenCalledWith({
      workspace_id: 'default',
      project_id: 'ideas',
      name: 'saved',
    });
    seed();
    await act(async () => {
      zone.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    });
    expect(state.ui.activeDate).toBeNull();
    expect(state.ui.activeTopic).toBe('ideas');
    await act(async () => root.unmount());
  });

  it('a press on the menu keeps it open, and a press on another row closes it', async () => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => root.render(createElement(Tree)));
    const button = host.querySelector('[data-node="ideas/plans"] button');
    await act(async () => {
      button.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 12, clientY: 24 }));
    });
    expect(document.querySelector('[data-role="folder-menu"]')).not.toBeNull();
    await act(async () => {
      document.querySelector('[data-role="folder-menu"]').dispatchEvent(new MouseEvent('pointerdown', { bubbles: true }));
    });
    expect(document.querySelector('[data-role="folder-menu"]')).not.toBeNull();
    await act(async () => {
      host.querySelector('[data-node="ideas/a"] button').dispatchEvent(new MouseEvent('pointerdown', { bubbles: true }));
    });
    expect(document.querySelector('[data-role="folder-menu"]')).toBeNull();
    await act(async () => root.unmount());
  });

  it('a folder at the depth limit has no New folder menu', async () => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => root.render(createElement(Tree)));
    const button = host.querySelector('[data-node="ideas/a/b/c"] button');
    expect(button).not.toBeNull();
    await act(async () => {
      button.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 3, clientY: 3 }));
    });
    const labels = [...document.querySelectorAll('[data-role="folder-menu"] button')].map((b) => b.textContent);
    expect(labels).toEqual(['Rename']);
    await act(async () => root.unmount());
  });

  it('right-click on an empty folder deletes that folder and can still do so at the depth limit', async () => {
    state.index.folders = [
      { project_id: 'ideas', path: 'saved' },
      { project_id: 'ideas', path: 'x/y/z' },
    ];
    state.ui.activeDate = dirKeyOf('ideas/saved');
    dirTreeStore.set({ expanded: pathKeys('ideas/x/y/z') });
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => root.render(createElement(Tree)));
    const saved = host.querySelector('[data-node="ideas/saved"] button');
    await act(async () => {
      saved.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 6, clientY: 6 }));
    });
    const labels = () => [...document.querySelectorAll('[data-role="folder-menu"] button')].map((b) => b.textContent);
    expect(labels()).toEqual(['New folder', 'Rename', 'Delete folder']);
    await act(async () => {
      [...document.querySelectorAll('[data-role="folder-menu"] button')].find((b) => b.textContent === 'Delete folder').dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    });
    expect(deleteFolder).toHaveBeenCalledTimes(1);
    expect(deleteFolder).toHaveBeenCalledWith({
      workspace_id: 'default',
      project_id: 'ideas',
      path: 'saved',
    });
    expect(refreshNotesIndex).toHaveBeenCalledTimes(1);
    expect(state.ui.activeDate).toBeNull();
    expect(state.ui.activeTopic).toBe('ideas');
    const deep = host.querySelector('[data-node="ideas/x/y/z"] button');
    await act(async () => {
      deep.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 9, clientY: 9 }));
    });
    expect(labels()).toEqual(['Rename', 'Delete folder']);
    await act(async () => root.unmount());
  });
});
