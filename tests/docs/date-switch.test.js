// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../frontend/src/notes/commands/cards.ts', () => ({ loadTitles: vi.fn() }));

import { state } from '../../frontend/src/host/state.ts';
import { selectDate } from '../../frontend/src/notes/commands/sidebar.ts';
import { applyCreateChrome } from '../../frontend/src/notes/commands/viewer/outlet.ts';
import { notesViewMode } from '../../frontend/src/notes/page.tsx';

const saved = {
  n1: { id: 'n1', project_id: 'ideas', created_at: '20260710120000', raw_path: 'a.md', title: '已保存' },
  n2: { id: 'n2', project_id: 'ideas', created_at: '20260711120000', raw_path: 'b.md', title: '另一天' },
};

function seed() {
  state.workspaces = {
    current: 'default',
    list: [{ workspace_id: 'default', workspace_name: 'Default', view_mode: 'timeline', is_default: true }],
  };
  state.index.data = { n1: { ...saved.n1 }, n2: { ...saved.n2 } };
  state.index.groupedByDate = [
    { date: '20260711', entries: [{ id: 'n2', entry: state.index.data.n2 }] },
    { date: '20260710', entries: [{ id: 'n1', entry: state.index.data.n1 }] },
  ];
  state.index.filteredGroups = state.index.groupedByDate;
  state.ui.activeTopic = 'ideas';
  state.ui.activeDate = '20260710';
  state.viewer.createSession = null;
  state.viewer.outletMode = '';
  state.viewer.editing = false;
  state.viewer.entry = null;
  window.location.hash = '#/spark?date=20260710';
}

function listRoute() {
  const params = new URLSearchParams(window.location.hash.split('?')[1] || '');
  return { date: params.get('date') || '', note: params.get('note') || '' };
}

beforeEach(() => {
  document.body.innerHTML = '<button id="btn-edit"></button><div id="note-outlet"></div>';
  seed();
});

describe('date switches after a new document', () => {
  it('re-clicking the current date while creating shows that day and keeps the saved document', () => {
    state.viewer.createSession = { tempId: 'tmp', status: 'creating' };
    state.viewer.outletMode = 'create';
    state.viewer.editing = true;
    applyCreateChrome();
    expect(document.getElementById('btn-edit').style.display).toBe('none');

    selectDate('20260710');

    expect(state.viewer.createSession).toBeNull();
    expect(state.viewer.outletMode).toBe('');
    expect(state.ui.activeDate).toBe('20260710');
    expect(notesViewMode(listRoute(), state.viewer)).toBe('list');
    expect(document.getElementById('btn-edit').style.display).toBe('');
    expect(state.index.data.n1).toMatchObject(saved.n1);
    expect(state.index.data.n2).toMatchObject(saved.n2);
  });

  it('switching to another date while creating shows that day and keeps both documents', () => {
    state.viewer.createSession = { tempId: 'tmp', status: 'creating' };
    state.viewer.outletMode = 'create';

    selectDate('20260711');

    expect(state.viewer.createSession).toBeNull();
    expect(notesViewMode(listRoute(), state.viewer)).toBe('list');
    expect(state.ui.activeDate).toBe('20260711');
    expect(listRoute().date).toBe('20260711');
    expect(state.index.data.n1.title).toBe('已保存');
    expect(state.index.data.n2.title).toBe('另一天');
  });

  it('re-clicking the open document’s date leaves the list and keeps the document', () => {
    window.location.hash = '#/spark?date=20260710&note=n1';
    state.viewer.outletMode = 'open';
    state.viewer.entry = state.index.data.n1;

    selectDate('20260710');

    expect(listRoute().note).toBe('');
    expect(notesViewMode(listRoute(), state.viewer)).toBe('list');
    expect(state.ui.activeDate).toBe('20260710');
    expect(state.index.data.n1).toMatchObject(saved.n1);
  });
});
