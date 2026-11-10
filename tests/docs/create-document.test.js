// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { describe, it, expect, beforeEach, vi } from 'vitest';

const { openCreateNote } = vi.hoisted(() => ({ openCreateNote: vi.fn() }));
vi.mock('../../frontend/src/notes/commands/viewer/create.ts', () => ({ openCreateNote }));
vi.mock('../../frontend/src/notes/commands/restore.ts', () => ({ enterProject: vi.fn() }));
vi.mock('../../frontend/src/notes/commands/sidebar.ts', async (importOriginal) => ({
  ...(await importOriginal()),
  selectDate: vi.fn(),
  selectTopic: vi.fn(),
}));

import { state } from '../../frontend/src/host/state.ts';
import { dirKeyOf } from '../../frontend/src/notes/state/dir-tree.ts';
import {
  cancelCreateProject,
  confirmCreateProject,
  startCreateDocument,
} from '../../frontend/src/notes/commands/create-document.ts';
import { landOnNewDocument } from '../../frontend/src/notes/commands/land-on-document.ts';
import { selectDate, selectTopic } from '../../frontend/src/notes/commands/sidebar.ts';
import { cancelTimelineCreate } from '../../frontend/src/notes/commands/timeline-create.ts';
import { createTargetStore } from '../../frontend/src/notes/state/create-target.ts';
import { timelineCreateStore } from '../../frontend/src/notes/state/timeline-create.ts';
import { CreateTargetDialog } from '../../frontend/src/notes/ui/create-target-dialog.tsx';

function seed({
  active = '',
  listed = null,
  titles = { ideas: 'Ideas', inbox: 'Inbox' },
  order = ['inbox', 'ideas'],
  view = 'timeline',
} = {}) {
  state.workspaces = {
    current: 'default',
    list: [{ workspace_id: 'default', workspace_name: 'Default', view_mode: view, is_default: true }],
  };
  state.index.topicTitles = titles;
  state.index.topicOrder = order;
  state.index.topicDefault = { inbox: true };
  state.ui.activeTopic = active;
  state.ui.activeDate = listed;
}

beforeEach(() => {
  openCreateNote.mockClear();
  vi.mocked(selectDate).mockClear();
  vi.mocked(selectTopic).mockClear();
  cancelCreateProject();
  cancelTimelineCreate();
  seed();
});

describe('startCreateDocument (the document page "+")', () => {
  it('timeline: the selected Project opens a title card and does not open the body editor', () => {
    seed({ active: 'ideas', listed: '20260710' });
    expect(startCreateDocument()).toBe(true);
    expect(createTargetStore.getSnapshot().open).toBe(false);
    expect(openCreateNote).not.toHaveBeenCalled();
    expect(timelineCreateStore.getSnapshot().project).toBe('ideas');
  });

  it('timeline All: the card uses the workspace default Project, not a chooser', () => {
    seed({ order: ['ideas', 'inbox'] });
    expect(startCreateDocument()).toBe(true);
    expect(openCreateNote).not.toHaveBeenCalled();
    expect(createTargetStore.getSnapshot().open).toBe(false);
    expect(timelineCreateStore.getSnapshot().project).toBe('inbox');
  });

  it('directory view: creates in the folder that is listed', () => {
    seed({ active: 'ideas', view: 'directory', listed: dirKeyOf('ideas/plans/q1') });
    startCreateDocument();
    expect(openCreateNote).toHaveBeenCalledWith(
      expect.objectContaining({ project_id: 'ideas', folder: 'plans/q1' }),
    );
  });

  it('directory view: the Project root is `/`, also when nothing or another Project is listed', () => {
    seed({ active: 'ideas', view: 'directory', listed: dirKeyOf('ideas') });
    startCreateDocument();
    seed({ active: 'ideas', view: 'directory', listed: null });
    startCreateDocument();
    seed({ active: 'ideas', view: 'directory', listed: dirKeyOf('inbox/plans') });
    startCreateDocument();
    expect(openCreateNote.mock.calls.map(([arg]) => arg.folder)).toEqual(['/', '/', '/']);
  });

  it('directory view with nothing selected: asks for a Project first, pre-selecting the first one', () => {
    seed({ view: 'directory', order: ['inbox', 'ideas'] });
    expect(startCreateDocument()).toBe(true);
    expect(openCreateNote).not.toHaveBeenCalled();
    expect(timelineCreateStore.getSnapshot().project).toBe('');
    expect(createTargetStore.getSnapshot()).toEqual({ open: true, project: 'inbox' });
  });

  it('confirming the chooser creates in the picked Project', () => {
    seed({ view: 'directory' });
    startCreateDocument();
    createTargetStore.set((p) => ({ ...p, project: 'ideas' }));
    confirmCreateProject();
    expect(createTargetStore.getSnapshot().open).toBe(false);
    expect(openCreateNote).toHaveBeenCalledWith(expect.objectContaining({ workspace_id: 'default', project_id: 'ideas' }));
  });

  it('cancelling the chooser creates nothing', () => {
    seed({ view: 'directory' });
    startCreateDocument();
    cancelCreateProject();
    expect(createTargetStore.getSnapshot().open).toBe(false);
    expect(openCreateNote).not.toHaveBeenCalled();
  });

  it('with no enabled Project there is nowhere to put a document: nothing happens', () => {
    seed({ titles: {}, order: [] });
    expect(startCreateDocument()).toBe(false);
    expect(createTargetStore.getSnapshot().open).toBe(false);
    expect(openCreateNote).not.toHaveBeenCalled();
  });

  it('a selected Project that is not enabled falls back to asking in the directory view', () => {
    seed({ active: 'gone', view: 'directory' });
    startCreateDocument();
    expect(openCreateNote).not.toHaveBeenCalled();
    expect(createTargetStore.getSnapshot().open).toBe(true);
  });
});

describe('chooser dialog', () => {
  it('renders only while open, lists projects in order, and Create confirms', async () => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => root.render(createElement(CreateTargetDialog)));
    expect(host.querySelector('#create-target-dialog')).toBeNull();

    await act(async () => {
      seed({ view: 'directory' });
      startCreateDocument();
    });
    expect(host.querySelector('#create-target-dialog')).not.toBeNull();
    await act(async () => {
      host.querySelector('#btn-create-target-confirm').click();
    });
    expect(host.querySelector('#create-target-dialog')).toBeNull();
    expect(openCreateNote).toHaveBeenCalledWith(expect.objectContaining({ project_id: 'inbox' }));
    await act(async () => root.unmount());
  });
});

describe('landOnNewDocument', () => {
  it('directory view: lists the folder the document was created in, the Project root included', () => {
    seed({ active: 'ideas', view: 'directory' });
    state.index.data = {
      n1: { project_id: 'ideas', raw_path: 'plans/q1/a.md' },
      n2: { project_id: 'ideas', raw_path: '202607101200-abc123.md' },
    };
    expect(landOnNewDocument('n1', 'ideas')).toBe(dirKeyOf('ideas/plans/q1'));
    expect(landOnNewDocument('n2', 'ideas')).toBe(dirKeyOf('ideas'));
  });

  it('timeline: selects the Project and the day the document was created', () => {
    state.index.data = { n1: { project_id: 'ideas', created_at: '20260710120000', raw_path: 'ideas/notes/a.md' } };
    expect(landOnNewDocument('n1', 'ideas')).toBe('20260710');
    expect(selectTopic).toHaveBeenCalledWith('ideas');
    expect(selectDate).toHaveBeenCalledWith('20260710');
  });

  it('unknown id: does nothing', () => {
    state.index.data = {};
    expect(landOnNewDocument('missing', 'ideas')).toBe('');
    expect(selectDate).not.toHaveBeenCalled();
  });
});
