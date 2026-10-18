// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const calls = [];
const entered = vi.hoisted(() => []);
const projects = { default: [], ws2: [] };
let deleteResult = { ok: true };

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchProjects: async (ws) => ({ projects: projects[ws] }),
  deleteProject: async (...a) => {
    calls.push(['delete', ...a]);
    if (deleteResult.throws) throw new Error(deleteResult.throws);
    return deleteResult;
  },
}));
vi.mock('../../frontend/src/notes/commands/restore.ts', () => ({
  enterProject: (id) => entered.push(id),
}));

const { state } = await import('../../frontend/src/host/state.ts');
const commands = await import('../../frontend/src/app-shell/commands/settings/notes-categories.ts');
const deleting = await import('../../frontend/src/app-shell/commands/settings/notes-categories-delete.ts');
const { notesCategoriesStore, emptyNotesCategoriesSnap } = await import(
  '../../frontend/src/app-shell/state/settings/notes-categories.ts'
);
const { NotesCategoriesEditHost } = await import('../../frontend/src/app-shell/ui/settings/notes-categories-edit.tsx');

const row = (id, title = id, extra = {}) => ({
  project_id: id,
  project_name: title,
  description: '',
  is_inbox: extra.is_inbox ?? id === 'inbox',
  doc_count: extra.doc_count ?? 0,
});

function mountPanel() {
  const host = document.createElement('div');
  document.body.appendChild(host);
  const root = createRoot(host);
  act(() => root.render(createElement(NotesCategoriesEditHost)));
  return { host, root };
}

describe('Deleting a project', () => {
  const rows = () => [
    row('inbox', 'Catch-all', { doc_count: 5 }),
    row('work', 'Work', { doc_count: 3 }),
    row('empty', 'Empty', { doc_count: 0 }),
    row('one', 'Single', { doc_count: 1 }),
  ];
  const deleteButtons = (host) => [...host.querySelectorAll('.notes-cat-row-delete')];
  const copy = (host) => host.querySelector('#notes-cat-delete-copy')?.textContent;
  const clickConfirm = async (host) => {
    await act(async () => host.querySelector('#btn-notes-cat-delete-confirm').click());
  };

  beforeEach(async () => {
    calls.length = 0;
    entered.length = 0;
    document.body.innerHTML = '';
    deleteResult = { ok: true };
    projects.default = rows();
    notesCategoriesStore.set(emptyNotesCategoriesSnap());
    state.workspaces.current = 'default';
    state.ui.activeTopic = null;
    await commands.selectProjectsWorkspace('default');
  });

  it('gives every row but the Built-in one a Delete button, left of Edit', () => {
    const { host, root } = mountPanel();
    const rowOf = (id) => host.querySelector(`.notes-cat-row[data-project="${id}"]`);
    expect(rowOf('inbox').querySelector('.notes-cat-row-delete')).toBeNull();
    for (const id of ['work', 'empty', 'one']) {
      const actions = [...rowOf(id).querySelectorAll('.notes-cat-row-actions button')].map((b) => b.textContent);
      expect(actions).toEqual(['Delete', 'Edit']);
    }
    expect(deleteButtons(host)).toHaveLength(3);
    act(() => root.unmount());
  });

  it('keeps Delete out of the click area that opens the editor, so no button nests in another', () => {
    const { host, root } = mountPanel();
    const open = host.querySelector('.notes-cat-row[data-project="work"] .notes-cat-row-open');
    expect(open.querySelector('button')).toBeNull();
    act(() => host.querySelector('.notes-cat-row[data-project="work"] .notes-cat-row-delete').click());
    expect(host.querySelector('#notes-cat-edit-dialog').classList.contains('open')).toBe(false);
    act(() => root.unmount());
  });

  it('asks first, naming the document count and the Built-in project by its current name', () => {
    const { host, root } = mountPanel();
    act(() => deleteButtons(host)[0].click());
    expect(host.querySelector('#notes-cat-delete-dialog').classList.contains('open')).toBe(true);
    expect(copy(host)).toBe(
      '3 documents in “Work” will be moved to “Catch-all”, then “Work” is deleted. This cannot be undone.',
    );
    expect(calls).toEqual([]);
    expect(host.querySelector('input')).toBeNull();
    expect(host.querySelector('#btn-notes-cat-delete-confirm').textContent).toBe('Delete');
    act(() => root.unmount());
  });

  it('words one document and no documents apart', () => {
    const { host, root } = mountPanel();
    act(() => host.querySelector('.notes-cat-row[data-project="one"] .notes-cat-row-delete').click());
    expect(copy(host)).toContain('1 document in “Single”');
    act(() => host.querySelector('#notes-cat-delete-dialog .btn-settings-ghost').click());
    act(() => host.querySelector('.notes-cat-row[data-project="empty"] .notes-cat-row-delete').click());
    expect(copy(host)).toBe('“Empty” has no documents. It will be deleted. This cannot be undone.');
    act(() => root.unmount());
  });

  it('Cancel closes the dialog and deletes nothing', () => {
    const { host, root } = mountPanel();
    act(() => deleteButtons(host)[0].click());
    act(() => host.querySelector('#notes-cat-delete-dialog .btn-settings-ghost').click());
    expect(host.querySelector('#notes-cat-delete-dialog').classList.contains('open')).toBe(false);
    expect(calls).toEqual([]);
    act(() => root.unmount());
  });

  it('Confirm deletes, closes the dialog and reloads the list', async () => {
    const { host, root } = mountPanel();
    act(() => deleteButtons(host)[0].click());
    projects.default = rows().filter((r) => r.project_id !== 'work');
    await clickConfirm(host);
    expect(calls).toEqual([['delete', 'default', 'work']]);
    expect(host.querySelector('#notes-cat-delete-dialog').classList.contains('open')).toBe(false);
    expect(notesCategoriesStore.getSnapshot().rows.map((r) => r.id)).toEqual(['inbox', 'empty', 'one']);
    act(() => root.unmount());
  });

  it('shows a failure in the dialog and leaves the list and the page alone', async () => {
    const { host, root } = mountPanel();
    act(() => deleteButtons(host)[0].click());
    deleteResult = { ok: false, error: 'Move failed, rolled back: disk full' };
    await clickConfirm(host);
    expect(host.querySelector('#notes-cat-delete-dialog').classList.contains('open')).toBe(true);
    expect(host.querySelector('#notes-cat-delete-error').textContent).toBe('Move failed, rolled back: disk full');
    expect(notesCategoriesStore.getSnapshot().rows.map((r) => r.id)).toEqual(['inbox', 'work', 'empty', 'one']);
    expect(host.querySelector('#notes-cat-list-error')).toBeNull();
    expect(host.querySelector('#btn-notes-cat-delete-confirm').disabled).toBe(false);

    deleteResult = { throws: 'network down' };
    await clickConfirm(host);
    expect(host.querySelector('#notes-cat-delete-error').textContent).toBe('network down');
    act(() => root.unmount());
  });

  it('ignores a second confirm while the first is still running', async () => {
    const { host, root } = mountPanel();
    act(() => deleteButtons(host)[0].click());
    const running = deleting.confirmDeleteProject('default');
    const second = await deleting.confirmDeleteProject('default');
    expect(second).toBe(false);
    await running;
    expect(calls.filter((c) => c[0] === 'delete')).toHaveLength(1);
    act(() => root.unmount());
  });

  it('never opens the dialog for the Built-in project', async () => {
    deleting.askDeleteProject('inbox');
    expect(notesCategoriesStore.getSnapshot().deleteId).toBe('');
  });

  it('reloads the page when the browsed workspace changed, and returns to the Built-in project if it was shown', async () => {
    const { host, root } = mountPanel();
    let reloads = 0;
    const onReload = () => {
      reloads += 1;
      document.dispatchEvent(new CustomEvent('cta:index-loaded'));
    };
    document.addEventListener('cta:reload', onReload);
    state.ui.activeTopic = 'work';
    act(() => deleteButtons(host)[0].click());
    projects.default = rows().filter((r) => r.project_id !== 'work');
    await clickConfirm(host);
    await act(async () => {});
    document.removeEventListener('cta:reload', onReload);
    expect(reloads).toBe(1);
    expect(entered).toEqual(['inbox']);
    act(() => root.unmount());
  });

  it('does not move the page to the Built-in project when it showed another one', async () => {
    const { host, root } = mountPanel();
    const onReload = () => document.dispatchEvent(new CustomEvent('cta:index-loaded'));
    document.addEventListener('cta:reload', onReload);
    state.ui.activeTopic = 'inbox';
    act(() => deleteButtons(host)[0].click());
    await clickConfirm(host);
    await act(async () => {});
    document.removeEventListener('cta:reload', onReload);
    expect(entered).toEqual([]);
    act(() => root.unmount());
  });

  it('leaves the page alone when another workspace lost the project', async () => {
    projects.ws2 = [row('inbox', 'Inbox'), row('plans', 'Plans', { doc_count: 2 })];
    await commands.selectProjectsWorkspace('ws2');
    const { host, root } = mountPanel();
    let reloads = 0;
    const onReload = () => (reloads += 1);
    document.addEventListener('cta:reload', onReload);
    act(() => deleteButtons(host)[0].click());
    await clickConfirm(host);
    document.removeEventListener('cta:reload', onReload);
    expect(calls).toEqual([['delete', 'ws2', 'plans']]);
    expect(reloads).toBe(0);
    act(() => root.unmount());
  });
});
