// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { createNote, createFolder, refreshNotesIndex } = vi.hoisted(() => ({
  createNote: vi.fn(),
  createFolder: vi.fn(),
  refreshNotesIndex: vi.fn(async () => true),
}));
vi.mock('../../frontend/src/host/api.ts', async (importOriginal) => ({
  ...(await importOriginal()),
  createNote,
  createFolder,
}));
vi.mock('../../frontend/src/notes/commands/reload-index.ts', () => ({ refreshNotesIndex }));
vi.mock('../../frontend/src/notes/commands/cards.ts', () => ({ loadTitles: vi.fn() }));

import { state } from '../../frontend/src/host/state.ts';
import { startCreateDocument } from '../../frontend/src/notes/commands/create-document.ts';
import { cancelDirCreate } from '../../frontend/src/notes/commands/dir-create.ts';
import { cancelFolderCreate } from '../../frontend/src/notes/commands/folder-create.ts';
import { selectDate } from '../../frontend/src/notes/commands/sidebar.ts';
import { cancelTimelineCreate } from '../../frontend/src/notes/commands/timeline-create.ts';
import { dirKeyOf } from '../../frontend/src/notes/state/dir-tree.ts';
import { DirTree } from '../../frontend/src/notes/ui/dir-tree.tsx';
import { FolderMenu, onDirectoryBackgroundClick, onDirectoryBackgroundMenu } from '../../frontend/src/notes/ui/folder-menu.tsx';
import { NotesDocList } from '../../frontend/src/notes/ui/cards.tsx';
import { notesViewMode } from '../../frontend/src/notes/page.tsx';

function setInput(input, value) {
  const proto = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value');
  proto.set.call(input, value);
  input.dispatchEvent(new Event('input', { bubbles: true }));
}

function workspace(view) {
  state.workspaces = {
    current: 'default',
    list: [{ workspace_id: 'default', workspace_name: 'Default', view_mode: view, is_default: true }],
  };
}

beforeEach(() => {
  createNote.mockReset();
  createFolder.mockReset();
  createNote.mockResolvedValue({ id: 'n-new', ok: true });
  createFolder.mockResolvedValue({ ok: true, path: 'saved' });
  refreshNotesIndex.mockClear();
  vi.spyOn(window, 'alert').mockImplementation(() => {});
  cancelTimelineCreate();
  cancelDirCreate();
  cancelFolderCreate();
  state.index.data = {
    kept: { id: 'kept', project_id: 'ideas', created_at: '20260710120000', raw_path: 'plans/a.md', title: '已保存' },
  };
  state.index.folders = [];
  state.index.topicTitles = { ideas: 'Ideas', inbox: 'Inbox' };
  state.index.topicOrder = ['inbox', 'ideas'];
  state.index.topicDefault = { inbox: true };
  state.index.groupedByDate = [
    { date: '20260710', entries: [{ id: 'kept', entry: state.index.data.kept }] },
  ];
  state.index.filteredGroups = state.index.groupedByDate;
  state.ui.activeTopic = '';
  state.ui.activeDate = '20260710';
  state.viewer.createSession = null;
  state.viewer.outletMode = '';
  workspace('timeline');
});

describe('inline create across timeline and directory', () => {
  it('timeline All creates an empty document from the title, and the title is not sent as the file name', async () => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => root.render(createElement(NotesDocList, { entries: [], date: '20260710' })));
    await act(async () => {
      startCreateDocument();
    });
    const input = host.querySelector('[aria-label="Document title"]');
    await act(async () => {
      setInput(input, 'Design notes');
      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    });
    expect(createNote).toHaveBeenCalledTimes(1);
    expect(createNote).toHaveBeenCalledWith({
      title: 'Design notes',
      source_type: 'jot',
      workspace_id: 'default',
      project_id: 'inbox',
      content: '',
    });
    expect(createNote.mock.calls[0][0].filename).toBeUndefined();
    expect(state.index.data.kept.title).toBe('已保存');
    await act(async () => root.unmount());
  });

  it('directory mode creates an empty file from the typed name, under the listed folder and at the project root', async () => {
    workspace('directory');
    state.ui.activeTopic = 'ideas';
    state.ui.activeDate = dirKeyOf('ideas/plans');
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    const zone = createElement(
      'div',
      { id: 'sidebar-tree-zone', onClick: onDirectoryBackgroundClick, onContextMenu: onDirectoryBackgroundMenu },
      createElement(DirTree),
      createElement(FolderMenu),
    );
    await act(async () => root.render(zone));

    await act(async () => {
      startCreateDocument();
    });
    let input = host.querySelector('[aria-label="Markdown file name"]');
    await act(async () => {
      setInput(input, 'architecture-notes.md');
      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    });
    expect(createNote).toHaveBeenCalledWith({
      title: 'architecture-notes',
      filename: 'architecture-notes.md',
      folder: 'plans',
      source_type: 'jot',
      workspace_id: 'default',
      project_id: 'ideas',
      content: '',
    });

    const blank = host.querySelector('#sidebar-tree-zone');
    await act(async () => {
      blank.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 6, clientY: 6 }));
    });
    await act(async () => {
      document.querySelector('[data-role="folder-menu"] button').dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    });
    input = host.querySelector('[aria-label="Folder name"]');
    await act(async () => {
      setInput(input, 'saved');
      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    });
    expect(createFolder).toHaveBeenCalledWith({
      workspace_id: 'default',
      project_id: 'ideas',
      name: 'saved',
    });
    expect(createNote).toHaveBeenCalledTimes(1);
    expect(state.index.data.kept.title).toBe('已保存');
    await act(async () => root.unmount());
  });

  it('an empty folder stays in the tree, and a date click returns to the list without dropping a saved document', async () => {
    workspace('directory');
    state.ui.activeTopic = 'ideas';
    state.index.data = {};
    state.index.folders = [{ project_id: 'ideas', path: 'saved' }];
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => root.render(createElement(DirTree)));
    const folder = host.querySelector('[data-node="ideas/saved"]');
    expect(folder).not.toBeNull();
    expect(folder.querySelector('.knowledge-doc-tree-name').textContent).toBe('saved');
    expect(host.querySelector('[data-kind="file"]')).toBeNull();
    await act(async () => root.unmount());

    workspace('timeline');
    state.index.data = {
      kept: { id: 'kept', project_id: 'ideas', created_at: '20260710120000', raw_path: 'plans/a.md', title: '已保存' },
    };
    state.ui.activeDate = '20260710';
    state.viewer.createSession = { tempId: 'tmp', status: 'creating' };
    state.viewer.outletMode = 'create';
    window.location.hash = '#/spark?date=20260710';
    selectDate('20260710');
    const params = new URLSearchParams(window.location.hash.split('?')[1] || '');
    expect(notesViewMode({ date: params.get('date') || '' }, state.viewer)).toBe('list');
    expect(state.index.data.kept.title).toBe('已保存');
  });
});
