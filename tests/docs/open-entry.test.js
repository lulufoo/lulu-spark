// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../frontend/src/notes/commands/cards.ts', () => ({ loadTitles: vi.fn() }));

import { state } from '../../frontend/src/host/state.ts';
import { openIndexEntry } from '../../frontend/src/notes/commands/open-entry.ts';
import { selectDate } from '../../frontend/src/notes/commands/sidebar.ts';
import { dirKeyOf } from '../../frontend/src/notes/state/dir-tree.ts';
import { notesViewMode } from '../../frontend/src/notes/page.tsx';

const folder = dirKeyOf('ideas/plans');

function seed() {
  state.workspaces = {
    current: 'default',
    list: [{ workspace_id: 'default', workspace_name: 'Default', view_mode: 'directory', is_default: true }],
  };
  state.index.data = {
    n1: {
      id: 'n1',
      _id: 'n1',
      project_id: 'ideas',
      raw_path: 'plans/q1/a.md',
      title: '笔记',
      layers: ['raw'],
    },
  };
  state.index.topicTitles = { ideas: 'Ideas' };
  state.index.topicOrder = ['ideas'];
  state.ui.activeTopic = 'ideas';
  state.ui.activeDate = folder;
  state.viewer.createSession = null;
  state.viewer.outletMode = '';
  state.viewer.editing = false;
  state.viewer.entry = null;
  window.location.hash = `#/spark?date=${encodeURIComponent(folder)}`;
}

function route() {
  const params = new URLSearchParams(window.location.hash.split('?')[1] || '');
  return { date: params.get('date') || '', note: params.get('note') || '' };
}

beforeEach(() => {
  document.body.innerHTML = '<div id="note-outlet"></div>';
  seed();
});

describe('opening a document from the tree', () => {
  it('a second click keeps the reader open and does not jump to the parent list', () => {
    expect(openIndexEntry('n1')).toBe(true);
    state.viewer.outletMode = 'open';
    state.viewer.entry = state.index.data.n1;
    expect(openIndexEntry('n1')).toBe(true);
    expect(route().note).toBe('n1');
    expect(state.viewer.outletMode).toBe('open');
    expect(notesViewMode(route(), state.viewer)).toBe('open');
  });

  it('clicking the parent folder still returns to the list', () => {
    window.location.hash = `#/spark?date=${encodeURIComponent(folder)}&note=n1`;
    state.viewer.outletMode = 'open';
    state.viewer.entry = state.index.data.n1;
    selectDate(folder);
    expect(route().note).toBe('');
    expect(state.viewer.outletMode).toBe('');
    expect(notesViewMode(route(), state.viewer)).toBe('list');
  });
});
