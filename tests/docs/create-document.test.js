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
import {
  cancelCreateProject,
  confirmCreateProject,
  startCreateDocument,
} from '../../frontend/src/notes/commands/create-document.ts';
import { landOnNewDocument } from '../../frontend/src/notes/commands/land-on-document.ts';
import { selectDate, selectTopic } from '../../frontend/src/notes/commands/sidebar.ts';
import { createTargetStore } from '../../frontend/src/notes/state/create-target.ts';
import { CreateTargetDialog } from '../../frontend/src/notes/ui/create-target-dialog.tsx';

function seed({ active = '', titles = { ideas: 'Ideas', inbox: 'Inbox' }, order = ['inbox', 'ideas'], view = 'timeline' } = {}) {
  state.workspaces = {
    current: 'default',
    list: [{ id: 'default', name: 'Default', view_mode: view, is_default: true }],
  };
  state.index.topicTitles = titles;
  state.index.topicOrder = order;
  state.ui.activeTopic = active;
}

beforeEach(() => {
  openCreateNote.mockClear();
  vi.mocked(selectDate).mockClear();
  vi.mocked(selectTopic).mockClear();
  cancelCreateProject();
  seed();
});

describe('startCreateDocument (the document page "+")', () => {
  it('uses the selected Project: no question, explicit workspace and project', () => {
    seed({ active: 'ideas' });
    expect(startCreateDocument()).toBe(true);
    expect(createTargetStore.getSnapshot().open).toBe(false);
    expect(openCreateNote).toHaveBeenCalledTimes(1);
    expect(openCreateNote).toHaveBeenCalledWith(
      expect.objectContaining({ workspace: 'default', project: 'ideas', temp_id: expect.any(String) }),
    );
  });

  it('timeline All: asks for a Project first, pre-selecting the first one in Settings order', () => {
    expect(startCreateDocument()).toBe(true);
    expect(openCreateNote).not.toHaveBeenCalled();
    expect(createTargetStore.getSnapshot()).toEqual({ open: true, project: 'inbox' });
  });

  it('confirming the chooser creates in the picked Project', () => {
    startCreateDocument();
    createTargetStore.set((p) => ({ ...p, project: 'ideas' }));
    confirmCreateProject();
    expect(createTargetStore.getSnapshot().open).toBe(false);
    expect(openCreateNote).toHaveBeenCalledWith(expect.objectContaining({ workspace: 'default', project: 'ideas' }));
  });

  it('cancelling the chooser creates nothing', () => {
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

  it('a selected Project that is not enabled falls back to asking', () => {
    seed({ active: 'gone' });
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
      startCreateDocument();
    });
    expect(host.querySelector('#create-target-dialog')).not.toBeNull();
    await act(async () => {
      host.querySelector('#btn-create-target-confirm').click();
    });
    expect(host.querySelector('#create-target-dialog')).toBeNull();
    expect(openCreateNote).toHaveBeenCalledWith(expect.objectContaining({ project: 'inbox' }));
    await act(async () => root.unmount());
  });
});

describe('landOnNewDocument', () => {
  it('timeline: selects the Project and the day the document was created', () => {
    state.index.data = { n1: { folder: 'ideas', created_at: '20260710120000', raw_path: 'ideas/notes/a.md' } };
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
