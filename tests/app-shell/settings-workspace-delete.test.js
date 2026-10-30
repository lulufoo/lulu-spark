// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const calls = [];
const projects = {};
let deleteResult = { ok: true };
let fetchProjectsError = '';

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchProjects: async (ws) => {
    calls.push(['projects', ws]);
    if (fetchProjectsError) return { error: fetchProjectsError };
    return { projects: projects[ws] ?? [] };
  },
  deleteWorkspace: async (...a) => {
    calls.push(['delete', ...a]);
    if (deleteResult.throws) throw new Error(deleteResult.throws);
    return deleteResult;
  },
  fetchDisabledWorkspaces: async () => ({ workspaces: [] }),
}));

const loadWorkspaces = vi.fn(async () => {
  const gone = calls.filter((c) => c[0] === 'delete').at(-1)?.[1];
  if (gone) {
    state.workspaces.list = state.workspaces.list.filter((w) => w.workspace_id !== gone);
    if (!state.workspaces.list.some((w) => w.workspace_id === state.workspaces.current)) {
      state.workspaces.current = state.workspaces.list.find((w) => w.is_default)?.workspace_id || '';
    }
  }
});
vi.mock('../../frontend/src/notes/commands/workspace.ts', () => ({ loadWorkspaces: () => loadWorkspaces() }));
vi.mock('../../frontend/src/notes/commands/workspace-switch.ts', () => ({ applyViewModeChange: () => {} }));

const { state } = await import('../../frontend/src/host/state.ts');
const { workspacesPanelStore, emptyWorkspacesPanelSnap } = await import(
  '../../frontend/src/app-shell/state/settings/workspaces-panel.ts'
);
const deleting = await import('../../frontend/src/app-shell/commands/settings/notes-workspaces-delete.ts');
const { NotesWorkspacesEditHost } = await import('../../frontend/src/app-shell/ui/settings/notes-workspaces-edit.tsx');

function row(id, name, extra = {}) {
  return { project_id: id, project_name: name, is_default: extra.is_default ?? id === 'inbox', doc_count: extra.doc_count ?? 0 };
}

function mountPanel() {
  const host = document.createElement('div');
  document.body.appendChild(host);
  const root = createRoot(host);
  act(() => root.render(createElement(NotesWorkspacesEditHost)));
  return { host, root };
}

const copy = (host) => host.querySelector('#notes-ws-delete-copy')?.textContent;
const clickConfirm = async (host) => {
  await act(async () => host.querySelector('#btn-notes-ws-delete-confirm').click());
};

describe('Deleting a workspace', () => {
  beforeEach(() => {
    calls.length = 0;
    fetchProjectsError = '';
    deleteResult = { ok: true };
    loadWorkspaces.mockClear();
    document.body.innerHTML = '';
    workspacesPanelStore.set(emptyWorkspacesPanelSnap());
    state.workspaces.current = 'ws2';
    state.workspaces.list = [
      { workspace_id: 'default', workspace_name: 'Default', view_mode: 'directory', is_default: true, description: '' },
      { workspace_id: 'ws2', workspace_name: 'Wiki', view_mode: 'timeline', is_default: false, description: 'Team wiki' },
    ];
    projects.ws2 = [row('inbox', 'Inbox', { doc_count: 2, is_default: true }), row('plans', 'Plans', { doc_count: 1 })];
    workspacesPanelStore.set({
      ...emptyWorkspacesPanelSnap(),
      disabled: [{ id: 'old', name: 'Old', sortIndex: 2 }],
    });
  });

  it('gives every enabled row but the Built-in one a Delete button, left of Edit', () => {
    const { host, root } = mountPanel();
    const rowOf = (id) => host.querySelector(`.notes-cat-row[data-workspace="${id}"]`);
    expect(rowOf('default').querySelector('.notes-cat-row-delete')).toBeNull();
    expect([...rowOf('ws2').querySelectorAll('.notes-cat-row-actions button')].map((b) => b.textContent)).toEqual(['Delete', 'Edit']);
    expect(host.querySelector('[data-workspace-disabled="old"] .notes-cat-row-delete')).toBeNull();
    expect(host.querySelector('[data-workspace-disabled="old"] .notes-cat-item-action').textContent).toBe('Enable');
    expect(host.querySelector('.reorder-handle')).toBeTruthy();
    act(() => root.unmount());
  });

  it('keeps Delete out of the click area that opens the editor', async () => {
    const { host, root } = mountPanel();
    const open = host.querySelector('.notes-cat-row[data-workspace="ws2"] .notes-cat-row-open');
    expect(open.querySelector('button')).toBeNull();
    await act(async () => host.querySelector('.notes-cat-row[data-workspace="ws2"] .notes-cat-row-delete').click());
    expect(host.querySelector('#notes-ws-edit-dialog').classList.contains('open')).toBe(false);
    expect(host.querySelector('#notes-ws-delete-dialog').classList.contains('open')).toBe(true);
    act(() => root.unmount());
  });

  it('asks first, naming the document count, project count and the default Inbox', async () => {
    const { host, root } = mountPanel();
    await act(async () => host.querySelector('.notes-cat-row-delete').click());
    expect(copy(host)).toBe(
      '3 documents in “Wiki” will be moved to “Inbox” in “Default”, then “Wiki” and its 2 projects are deleted. This cannot be undone.',
    );
    expect(host.querySelector('input')).toBeNull();
    expect(host.querySelector('#btn-notes-ws-delete-confirm').textContent).toBe('Delete');
    act(() => root.unmount());
  });

  it('shows a fetch failure on the list and does not open a lying dialog', async () => {
    fetchProjectsError = 'down';
    const { host, root } = mountPanel();
    await act(async () => host.querySelector('.notes-cat-row-delete').click());
    expect(host.querySelector('#notes-ws-delete-dialog').classList.contains('open')).toBe(false);
    expect(host.querySelector('#notes-workspace-list-error').textContent).toBe('down');
    act(() => root.unmount());
  });

  it('Confirm deletes, closes the dialog and falls back when the current workspace is gone', async () => {
    const { host, root } = mountPanel();
    let reloads = 0;
    const onReload = () => {
      reloads += 1;
    };
    document.addEventListener('cta:reload', onReload);
    await act(async () => host.querySelector('.notes-cat-row-delete').click());
    await clickConfirm(host);
    document.removeEventListener('cta:reload', onReload);
    expect(calls.filter((c) => c[0] === 'delete')).toEqual([['delete', 'ws2']]);
    expect(host.querySelector('#notes-ws-delete-dialog').classList.contains('open')).toBe(false);
    expect(state.workspaces.current).toBe('default');
    expect(reloads).toBe(1);
    act(() => root.unmount());
  });

  it('shows a failure in the dialog and leaves the workspace in the list', async () => {
    const { host, root } = mountPanel();
    await act(async () => host.querySelector('.notes-cat-row-delete').click());
    deleteResult = { ok: false, error: 'Moved 1 of 3 documents; stopped at abc' };
    await clickConfirm(host);
    expect(host.querySelector('#notes-ws-delete-dialog').classList.contains('open')).toBe(true);
    expect(host.querySelector('#notes-ws-delete-error').textContent).toBe('Moved 1 of 3 documents; stopped at abc');
    expect(state.workspaces.list.map((w) => w.workspace_id)).toEqual(['default', 'ws2']);
    expect(host.querySelector('#btn-notes-ws-delete-confirm').disabled).toBe(false);
    act(() => root.unmount());
  });

  it('ignores a second confirm while the first is still running', async () => {
    const { host, root } = mountPanel();
    await act(async () => host.querySelector('.notes-cat-row-delete').click());
    const running = deleting.confirmDeleteWorkspace();
    const second = await deleting.confirmDeleteWorkspace();
    expect(second).toBe(false);
    await running;
    expect(calls.filter((c) => c[0] === 'delete')).toHaveLength(1);
    act(() => root.unmount());
  });

  it('never opens the dialog for the Built-in workspace', async () => {
    await deleting.askDeleteWorkspace('default');
    expect(workspacesPanelStore.getSnapshot().deleteId).toBe('');
  });
});
