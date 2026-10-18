// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const calls = [];
const projects = { default: [], ws2: [] };

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchProjects: async (ws) => ({ projects: projects[ws] }),
  createProject: async (...a) => (calls.push(['create', ...a]), { ok: true }),
  updateProject: async (...a) => (calls.push(['update', ...a]), { ok: true }),
}));

const { state } = await import('../../frontend/src/host/state.ts');
const commands = await import('../../frontend/src/app-shell/commands/settings/notes-categories.ts');
const { notesCategoriesStore, emptyNotesCategoriesSnap } = await import(
  '../../frontend/src/app-shell/state/settings/notes-categories.ts'
);
const { WorkspaceNavItems } = await import('../../frontend/src/app-shell/ui/settings/workspace-nav.tsx');
const { NotesCategoriesEditHost } = await import('../../frontend/src/app-shell/ui/settings/notes-categories-edit.tsx');
const tabs = await import('../../frontend/src/app-shell/ui/settings/tabs.ts');

const row = (id, title = id, extra = {}) => ({
  project_id: id,
  project_name: title,
  description: '',
  is_inbox: extra.is_inbox ?? id === 'inbox',
});

describe('Workspace projects settings', () => {
  beforeEach(() => {
    calls.length = 0;
    projects.default = [row('inbox', 'Inbox'), row('work', 'Work')];
    projects.ws2 = [row('inbox', 'Inbox'), row('plans', 'Plans')];
    notesCategoriesStore.set(emptyNotesCategoriesSnap());
    state.workspaces.current = 'default';
    state.index.topicTitles = {};
    state.index.topicOrder = [];
  });

  it('acts on the selected workspace, not the one the page browses', async () => {
    await commands.selectProjectsWorkspace('ws2');
    expect(notesCategoriesStore.getSnapshot().rows.map((r) => r.id)).toEqual(['inbox', 'plans']);
    await commands.addNotesCategory('ws2', 'Ideas', '');
    commands.openNotesCategoryEditor('plans');
    commands.setNotesCategoryDraft('Roadmap', '');
    await commands.saveNotesCategory('ws2');
    expect(calls).toEqual([['create', 'ws2', 'Ideas', ''], ['update', 'ws2', 'plans', 'Roadmap', '']]);
    // The page browses `default`, so its title map is untouched by ws2 edits.
    expect(state.index.topicTitles).toEqual({});
  });

  it('keeps the browsed workspace titles in step when it is the one being edited', async () => {
    await commands.selectProjectsWorkspace('default');
    expect(state.index.topicTitles).toEqual({ inbox: 'Inbox', work: 'Work' });
    expect(state.index.topicOrder).toEqual(['inbox', 'work']);
  });

  it('still treats a renamed inbox as Built-in when its id is not inbox', async () => {
    projects.default = [row('minted', 'Catch-all', { is_inbox: true }), row('work', 'Work')];
    await commands.selectProjectsWorkspace('default');
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    act(() => root.render(createElement(NotesCategoriesEditHost)));
    const inbox = host.querySelector('button.notes-cat-item[data-project="minted"]');
    expect(inbox.querySelector('.notes-cat-badge').textContent).toBe('Built-in');
    act(() => inbox.click());
    const labels = () => [...host.querySelectorAll('.notes-cat-edit-actions button')].map((b) => b.textContent);
    expect(labels()).toEqual(['Cancel', 'Save']);
    act(() => root.unmount());
  });

  it('drops a late answer for a workspace that is no longer selected', async () => {
    const first = commands.selectProjectsWorkspace('default');
    await commands.selectProjectsWorkspace('ws2');
    await first;
    expect(notesCategoriesStore.getSnapshot().workspaceId).toBe('ws2');
    expect(notesCategoriesStore.getSnapshot().rows.map((r) => r.id)).toEqual(['inbox', 'plans']);
  });

  it('maps every workspace nav item onto the single workspace panel', () => {
    document.body.innerHTML = `
      <button class="settings-nav-item" data-panel="notes"></button>
      <button class="settings-nav-item" data-panel="workspace:ws2"></button>
      <div class="settings-panel" id="settings-panel-notes"></div>
      <div class="settings-panel" id="settings-panel-workspace"></div>`;
    expect(tabs.isWorkspacePanel('workspace:ws2')).toBe(true);
    expect(tabs.workspaceIdOfPanel('workspace:ws2')).toBe('ws2');
    tabs.switchPanel('workspace:ws2');
    expect(document.getElementById('settings-panel-workspace').classList.contains('active')).toBe(true);
    expect(document.getElementById('settings-panel-notes').classList.contains('active')).toBe(false);
    expect(document.querySelector('[data-panel="workspace:ws2"]').classList.contains('active')).toBe(true);
  });

  it('paints one nav item per workspace and follows renames', async () => {
    const { notifyState } = await import('../../frontend/src/host/state.ts');
    state.workspaces.list = [
      { workspace_id: 'default', workspace_name: 'Default', view_mode: 'directory', is_default: true },
      { workspace_id: 'ws2', workspace_name: 'Knowledge', view_mode: 'timeline', is_default: false },
    ];
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    act(() => root.render(createElement(WorkspaceNavItems)));
    const items = () => [...host.querySelectorAll('button')].map((b) => [b.dataset.panel, b.textContent]);
    expect(items()).toEqual([['workspace:default', 'Default'], ['workspace:ws2', 'Knowledge']]);
    state.workspaces.list = state.workspaces.list.map((w) => (w.workspace_id === 'ws2' ? { ...w, workspace_name: 'Wiki' } : w));
    act(() => notifyState());
    expect(items()[1]).toEqual(['workspace:ws2', 'Wiki']);
    act(() => root.unmount());
  });

  it('lists whole-row project buttons that open the dialog; the dialog only edits', async () => {
    await commands.selectProjectsWorkspace('default');
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    act(() => root.render(createElement(NotesCategoriesEditHost)));
    const inbox = host.querySelector('button.notes-cat-item[data-project="inbox"]');
    const work = host.querySelector('button.notes-cat-item[data-project="work"]');
    expect(inbox.querySelector('.notes-cat-item-action').textContent).toBe('Edit');
    expect(work.querySelector('.notes-cat-item-action').textContent).toBe('Edit');
    expect(host.querySelector('#notes-cat-edit-dialog').classList.contains('open')).toBe(false);

    act(() => work.click());
    expect(host.querySelector('#notes-cat-edit-dialog').classList.contains('open')).toBe(true);
    expect(host.querySelector('#notes-cat-edit-dialog-title').textContent).toBe('Edit project');
    const labels = () => [...host.querySelectorAll('.notes-cat-edit-actions button')].map((b) => b.textContent);
    expect(labels()).toEqual(['Cancel', 'Save']);
    expect(calls).toEqual([]);

    act(() => inbox.click());
    expect(labels()).toEqual(['Cancel', 'Save']);
    act(() => root.unmount());
  });

  it('has no Disabled list and no Enable or Disable button', async () => {
    await commands.selectProjectsWorkspace('default');
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    act(() => root.render(createElement(NotesCategoriesEditHost)));
    act(() => host.querySelector('button.notes-cat-item[data-project="work"]').click());
    expect(host.querySelector('details#notes-projects-disabled')).toBeNull();
    expect(host.textContent).not.toMatch(/Disable|Enable/);
    act(() => root.unmount());
  });
});
