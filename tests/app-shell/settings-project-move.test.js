// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const calls = [];
const cancelCalls = [];
const projects = { default: [], other: [] };
const workspaces = [
  { workspace_id: 'default', workspace_name: 'Agent' },
  { workspace_id: 'other', workspace_name: 'Other' },
];
let moveResult = { ok: true };
let holdMove = null;

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchWorkspaces: async () => ({ workspaces }),
  fetchProjects: async (ws) => ({ projects: projects[ws] || [] }),
  moveProject: async (...args) => {
    calls.push(args);
    if (holdMove) await holdMove;
    if (moveResult.throws) throw new Error(moveResult.throws);
    return moveResult;
  },
  cancelMoveProject: async () => {
    cancelCalls.push(true);
    return { ok: true };
  },
}));
vi.mock('../../frontend/src/notes/commands/restore.ts', () => ({
  enterProject: () => {},
}));

const { state } = await import('../../frontend/src/host/state.ts');
const commands = await import('../../frontend/src/app-shell/commands/settings/notes-categories.ts');
const { notesCategoriesStore, emptyNotesCategoriesSnap } = await import(
  '../../frontend/src/app-shell/state/settings/notes-categories.ts'
);
const { NotesCategoriesEditHost } = await import('../../frontend/src/app-shell/ui/settings/notes-categories-edit.tsx');
const { describeProjectMove } = await import(
  '../../frontend/src/app-shell/ui/settings/notes-categories-move-dialog.tsx'
);

const row = (id, title = id, extra = {}) => ({
  project_id: id,
  project_name: title,
  description: extra.description || '',
  is_default: extra.is_default ?? id === 'inbox',
  doc_count: extra.doc_count ?? 0,
});

function mountPanel() {
  const host = document.createElement('div');
  document.body.appendChild(host);
  const root = createRoot(host);
  act(() => root.render(createElement(NotesCategoriesEditHost)));
  return { host, root };
}

describe('Moving a project', () => {
  beforeEach(async () => {
    calls.length = 0;
    cancelCalls.length = 0;
    document.body.innerHTML = '';
    moveResult = { ok: true };
    holdMove = null;
    projects.default = [
      row('inbox', 'Inbox', { doc_count: 1 }),
      row('work', 'Work', { doc_count: 3 }),
    ];
    projects.other = [row('shelf', 'Shelf', { description: 'Kept' })];
    notesCategoriesStore.set(emptyNotesCategoriesSnap());
    state.workspaces.current = 'default';
    state.ui.activeTopic = null;
    await commands.selectProjectsWorkspace('default');
  });

  it('hides Move on the Built-in project and puts it left of Delete', () => {
    const { host } = mountPanel();
    const rowOf = (id) => host.querySelector(`.notes-cat-row[data-project="${id}"]`);
    expect(rowOf('inbox').querySelector('.notes-cat-row-move')).toBeNull();
    const actions = [...rowOf('work').querySelectorAll('.notes-cat-row-actions button')].map((b) => b.textContent);
    expect(actions).toEqual(['Move', 'Delete', 'Edit']);
  });

  it('asks before moving, and keeps the Built-in project in the same workspace', async () => {
    const { host } = mountPanel();
    await act(async () => host.querySelector('.notes-cat-row-move').click());
    expect(calls).toEqual([]);
    const names = [...host.querySelectorAll('#notes-cat-move-list button')].map((b) => b.textContent);
    expect(names).toEqual(['Inbox']);
    await act(async () => host.querySelector('#notes-cat-move-list button').click());
    expect(host.querySelector('#notes-cat-move-copy').textContent).toBe(
      describeProjectMove('Work', 3, 'Inbox', 'Agent'),
    );
    expect(calls).toEqual([]);
    await act(async () => host.querySelector('#btn-notes-cat-move-confirm').click());
    expect(calls).toEqual([['default', 'work', 'default', 'inbox']]);
  });

  it('lists projects of another workspace, including its Built-in project', async () => {
    const { host } = mountPanel();
    await act(async () => host.querySelector('.notes-cat-row-move').click());
    await act(async () => host.querySelector('#notes-cat-move-workspaces button:last-child').click());
    const names = [...host.querySelectorAll('#notes-cat-move-list button')].map((b) => b.textContent);
    expect(names.join(' ')).toContain('Shelf');
    expect(names.join(' ')).toContain('Kept');
  });

  it('keeps the dialog open and shows the error when the move fails', async () => {
    moveResult = { error: 'Moved 1 of 3 documents; stopped at doc-2: blocked' };
    const { host } = mountPanel();
    await act(async () => host.querySelector('.notes-cat-row-move').click());
    await act(async () => host.querySelector('#notes-cat-move-list button').click());
    await act(async () => host.querySelector('#btn-notes-cat-move-confirm').click());
    expect(host.querySelector('#notes-cat-move-dialog').classList.contains('open')).toBe(true);
    expect(host.querySelector('#notes-cat-move-error').textContent).toContain('stopped at doc-2');
  });

  it('stays open and loading until cancel stops the move, and a blank click does not close it', async () => {
    let release;
    holdMove = new Promise((resolve) => {
      release = resolve;
    });
    moveResult = { ok: true, cancelled: true, moved: 1, total: 4 };
    const { host } = mountPanel();
    await act(async () => host.querySelector('.notes-cat-row-move').click());
    await act(async () => host.querySelector('#notes-cat-move-list button').click());
    await act(async () => {
      host.querySelector('#btn-notes-cat-move-confirm').click();
    });
    expect(host.querySelector('#notes-cat-move-dialog').classList.contains('open')).toBe(true);
    expect(host.textContent).toContain('Moving…');
    await act(async () => host.querySelector('#notes-cat-move-dialog').click());
    expect(host.querySelector('#notes-cat-move-dialog').classList.contains('open')).toBe(true);
    expect(cancelCalls).toEqual([]);
    await act(async () => host.querySelector('#notes-cat-move-dialog .btn-settings-ghost').click());
    expect(cancelCalls).toEqual([true]);
    release();
    await act(async () => {
      await holdMove;
    });
    expect(host.querySelector('#notes-cat-move-dialog').classList.contains('open')).toBe(true);
    expect(host.querySelector('#notes-cat-move-error').textContent).toContain('Stopped after moving 1 of 4');
  });
});

describe('describeProjectMove', () => {
  it('names the target project and says the source is deleted', () => {
    expect(describeProjectMove('Work', 0, 'Shelf', 'Other')).toBe(
      '“Work” has no documents. It will be deleted. This cannot be undone.',
    );
    expect(describeProjectMove('Work', 1, 'Shelf', 'Other')).toBe(
      '1 document in “Work” will be moved to “Shelf” in “Other”, then “Work” is deleted. This cannot be undone.',
    );
    expect(describeProjectMove('Work', 2, 'Shelf', 'Other')).toContain('2 documents');
  });
});
