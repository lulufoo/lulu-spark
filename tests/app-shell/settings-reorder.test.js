// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const calls = [];
let failWith = '';
let workspaceDisabled = [];
let projectRows = [];

vi.mock('../../frontend/src/host/api.ts', () => ({
  reorderWorkspaces: async (ids) => reply(['reorderWorkspaces', ids]),
  reorderProjects: async (ws, folders) => reply(['reorderProjects', ws, folders]),
  fetchDisabledWorkspaces: async () => ({ workspaces: workspaceDisabled }),
  fetchProjects: async () => ({ projects: projectRows }),
}));
vi.mock('../../frontend/src/notes/commands/workspace.ts', () => ({ loadWorkspaces: async () => {} }));
vi.mock('../../frontend/src/notes/commands/workspace-switch.ts', () => ({ applyViewModeChange: () => {} }));

function reply(call) {
  calls.push(call);
  if (failWith) {
    const error = failWith;
    failWith = '';
    return { error };
  }
  return { ok: true };
}

const { state } = await import('../../frontend/src/host/state.ts');
const workspaces = await import('../../frontend/src/app-shell/commands/settings/workspaces.ts');
const categories = await import('../../frontend/src/app-shell/commands/settings/notes-categories.ts');
const { workspacesPanelStore, emptyWorkspacesPanelSnap } = await import(
  '../../frontend/src/app-shell/state/settings/workspaces-panel.ts'
);
const { notesCategoriesStore, emptyNotesCategoriesSnap } = await import(
  '../../frontend/src/app-shell/state/settings/notes-categories.ts'
);
const { ReorderRows } = await import('../../frontend/src/shared/reorder-rows.tsx');
const { NotesWorkspacesEditHost } = await import('../../frontend/src/app-shell/ui/settings/notes-workspaces-edit.tsx');

const ws = (id, sort_index) => ({ workspace_id: id, workspace_name: id, view_mode: 'directory', is_default: id === 'default', icon: '', sort_index });
const project = (id, sort_index, extra = {}) => ({
  project_id: id,
  project_name: id,
  description: '',
  icon: '',
  sort_index,
  is_inbox: extra.is_inbox ?? id === 'inbox',
});
const ids = () => state.workspaces.list.map((w) => w.workspace_id);

beforeEach(() => {
  calls.length = 0;
  failWith = '';
  workspaceDisabled = [{ workspace_id: 'off', workspace_name: 'Off', sort_index: 1 }];
  projectRows = [project('inbox', 0), project('a', 1), project('b', 2)];
  state.workspaces.list = [ws('default', 0), ws('x', 2), ws('y', 3)];
  workspacesPanelStore.set({ ...emptyWorkspacesPanelSnap(), disabled: [{ id: 'off', name: 'Off', sortIndex: 1 }] });
  notesCategoriesStore.set(emptyNotesCategoriesSnap());
  document.body.innerHTML = '';
});

describe('reordering workspaces', () => {
  it('shows the new order at once and sends every id, the disabled one in its place', async () => {
    const done = workspaces.reorderWorkspaceList(['y', 'default', 'x']);
    expect(ids()).toEqual(['y', 'default', 'x']);
    expect(await done).toBe(true);
    expect(calls).toEqual([['reorderWorkspaces', ['y', 'off', 'default', 'x']]]);
  });

  it('puts the old order back and says why when the backend refuses', async () => {
    failWith = 'boom';
    expect(await workspaces.reorderWorkspaceList(['y', 'default', 'x'])).toBe(false);
    expect(ids()).toEqual(['default', 'x', 'y']);
    expect(workspacesPanelStore.getSnapshot().error).toBe('Could not change the order: boom');
  });

  it('ignores a list that does not name every shown workspace', async () => {
    expect(await workspaces.reorderWorkspaceList(['x'])).toBe(false);
    expect(calls).toEqual([]);
    expect(ids()).toEqual(['default', 'x', 'y']);
  });
});

describe('reordering projects', () => {
  const rows = () => notesCategoriesStore.getSnapshot().rows.map((r) => r.id);

  it('shows the new order at once and sends every id', async () => {
    await categories.selectProjectsWorkspace('default');
    const done = categories.reorderNotesCategories('default', ['b', 'inbox', 'a']);
    expect(rows()).toEqual(['b', 'inbox', 'a']);
    expect(await done).toBe(true);
    expect(calls).toEqual([['reorderProjects', 'default', ['b', 'inbox', 'a']]]);
  });

  it('rolls back on a refusal', async () => {
    await categories.selectProjectsWorkspace('default');
    failWith = 'nope';
    expect(await categories.reorderNotesCategories('default', ['b', 'inbox', 'a'])).toBe(false);
    expect(rows()).toEqual(['inbox', 'a', 'b']);
    expect(notesCategoriesStore.getSnapshot().error).toBe('Could not change the order: nope');
  });
});

describe('the reorder rows', () => {
  function mount(rowIds, onReorder) {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    act(() =>
      root.render(
        createElement(ReorderRows, {
          onReorder,
          rows: rowIds.map((id) => ({ id, content: createElement('button', { 'data-row': id }, id) })),
        }),
      ),
    );
    // Rows are 20px tall, stacked from the top.
    [...host.querySelectorAll('.reorder-row')].forEach((el, i) => {
      el.getBoundingClientRect = () => ({ top: i * 20, height: 20, bottom: i * 20 + 20, left: 0, right: 100, width: 100 });
    });
    return host;
  }
  const fire = (target, type, y) =>
    act(() => {
      target.dispatchEvent(new MouseEvent(type, { bubbles: true, clientY: y, button: 0 }));
    });

  it('a list of one row has no handle', () => {
    const host = mount(['only'], () => {});
    expect(host.querySelectorAll('.reorder-handle')).toHaveLength(0);
  });

  it('dragging a handle past another row reports the new order and marks the drop place', () => {
    const seen = [];
    const host = mount(['a', 'b', 'c'], (next) => seen.push(next));
    const handle = host.querySelector('[data-reorder-id="a"] .reorder-handle');
    fire(handle, 'pointerdown', 10);
    expect(host.querySelector('[data-reorder-id="a"]').classList.contains('is-dragging')).toBe(true);
    fire(window, 'pointermove', 55);
    expect(host.querySelector('[data-reorder-id="c"]').classList.contains('drop-after')).toBe(true);
    fire(window, 'pointerup', 55);
    expect(seen).toEqual([['b', 'c', 'a']]);
    expect(host.querySelector('.is-dragging')).toBeNull();
    expect(host.querySelector('.drop-before, .drop-after')).toBeNull();
  });

  it('drops before a row when released above its middle', () => {
    const seen = [];
    const host = mount(['a', 'b', 'c'], (next) => seen.push(next));
    fire(host.querySelector('[data-reorder-id="c"] .reorder-handle'), 'pointerdown', 50);
    fire(window, 'pointermove', 5);
    expect(host.querySelector('[data-reorder-id="a"]').classList.contains('drop-before')).toBe(true);
    fire(window, 'pointerup', 5);
    expect(seen).toEqual([['c', 'a', 'b']]);
  });

  it('releasing where the row already is, or cancelling, changes nothing', () => {
    const seen = [];
    const host = mount(['a', 'b', 'c'], (next) => seen.push(next));
    const handle = host.querySelector('[data-reorder-id="b"] .reorder-handle');
    fire(handle, 'pointerdown', 30);
    fire(window, 'pointerup', 30);
    fire(handle, 'pointerdown', 30);
    fire(window, 'pointermove', 5);
    fire(window, 'pointercancel', 5);
    fire(window, 'pointerup', 5);
    expect(seen).toEqual([]);
  });

  it('clicking the row body never starts a drag', () => {
    const seen = [];
    const host = mount(['a', 'b'], (next) => seen.push(next));
    fire(host.querySelector('[data-row="a"]'), 'pointerdown', 10);
    fire(window, 'pointermove', 35);
    fire(window, 'pointerup', 35);
    expect(seen).toEqual([]);
  });
});

describe('the workspace list', () => {
  it('has a handle on every row and the rows keep their edit buttons', () => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    act(() => root.render(createElement(NotesWorkspacesEditHost)));
    expect(host.querySelectorAll('.reorder-handle')).toHaveLength(3);
    expect(host.querySelectorAll('button.notes-cat-item[data-workspace]')).toHaveLength(3);
    act(() => root.unmount());
  });

  it('shows no handle when only one workspace is enabled', () => {
    state.workspaces.list = [ws('default', 0)];
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    act(() => root.render(createElement(NotesWorkspacesEditHost)));
    expect(host.querySelectorAll('.reorder-handle')).toHaveLength(0);
    act(() => root.unmount());
  });
});
