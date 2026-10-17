// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const calls = [];
const projects = { default: [], ws2: [] };
const disabled = { default: [], ws2: [] };

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchProjects: async (ws) => ({ projects: projects[ws] }),
  fetchDisabledProjects: async (ws) => ({ projects: disabled[ws] }),
  createProject: async (...a) => (calls.push(['create', ...a]), { ok: true }),
  updateProject: async (...a) => (calls.push(['update', ...a]), { ok: true }),
  disableProject: async (...a) => (calls.push(['disable', ...a]), { ok: true }),
  enableProject: async (...a) => (calls.push(['enable', ...a]), { ok: true }),
}));

const { state } = await import('../../frontend/src/host/state.ts');
const commands = await import('../../frontend/src/app-shell/commands/settings/notes-categories.ts');
const { notesCategoriesStore, emptyNotesCategoriesSnap } = await import(
  '../../frontend/src/app-shell/state/settings/notes-categories.ts'
);
const { WorkspaceNavItems } = await import('../../frontend/src/app-shell/ui/settings/workspace-nav.tsx');
const { WorkspaceProjectList } = await import('../../frontend/src/app-shell/ui/settings/workspace-project-list.tsx');
const tabs = await import('../../frontend/src/app-shell/ui/settings/tabs.ts');

const row = (folder, title = folder) => ({ folder, title, description: '' });

describe('Workspace projects settings', () => {
  beforeEach(() => {
    calls.length = 0;
    projects.default = [row('inbox', 'Inbox'), row('work', 'Work')];
    projects.ws2 = [row('inbox', 'Inbox'), row('plans', 'Plans')];
    disabled.default = [row('old', 'Old')];
    disabled.ws2 = [];
    notesCategoriesStore.set(emptyNotesCategoriesSnap());
    state.workspaces.current = 'default';
    state.index.topicTitles = {};
  });

  it('acts on the selected workspace, not the one the page browses', async () => {
    await commands.selectProjectsWorkspace('ws2');
    expect(notesCategoriesStore.getSnapshot().rows.map((r) => r.id)).toEqual(['inbox', 'plans']);
    await commands.addNotesCategory('ws2', 'Ideas', '');
    await commands.disableNotesCategory('ws2', 'plans');
    expect(calls).toEqual([['create', 'ws2', 'Ideas', ''], ['disable', 'ws2', 'plans']]);
    // The page browses `default`, so its title map is untouched by ws2 edits.
    expect(state.index.topicTitles).toEqual({});
  });

  it('keeps the browsed workspace titles in step when it is the one being edited', async () => {
    await commands.selectProjectsWorkspace('default');
    expect(state.index.topicTitles).toEqual({ inbox: 'Inbox', work: 'Work' });
  });

  it('lists disabled projects apart and enables them again', async () => {
    await commands.selectProjectsWorkspace('default');
    expect(notesCategoriesStore.getSnapshot().disabled.map((r) => r.id)).toEqual(['old']);
    await commands.enableNotesCategory('default', 'old');
    expect(calls).toEqual([['enable', 'default', 'old']]);
  });

  it('never offers Disable for inbox', () => {
    commands.askNotesCategoryDisable('inbox');
    expect(notesCategoriesStore.getSnapshot().confirmDisableId).toBe('');
    commands.askNotesCategoryDisable('work');
    expect(notesCategoriesStore.getSnapshot().confirmDisableId).toBe('work');
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
      { id: 'default', name: 'Default', view_mode: 'directory', is_default: true },
      { id: 'ws2', name: 'Knowledge', view_mode: 'timeline', is_default: false },
    ];
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    act(() => root.render(createElement(WorkspaceNavItems)));
    const items = () => [...host.querySelectorAll('button')].map((b) => [b.dataset.panel, b.textContent]);
    expect(items()).toEqual([['workspace:default', 'Default'], ['workspace:ws2', 'Knowledge']]);
    state.workspaces.list = state.workspaces.list.map((w) => (w.id === 'ws2' ? { ...w, name: 'Wiki' } : w));
    act(() => notifyState());
    expect(items()[1]).toEqual(['workspace:ws2', 'Wiki']);
    act(() => root.unmount());
  });

  it('shows Edit and Disable per project, and no Disable for inbox', async () => {
    await commands.selectProjectsWorkspace('default');
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    act(() => root.render(createElement(WorkspaceProjectList)));
    const inbox = host.querySelector('[data-project="inbox"]');
    const work = host.querySelector('[data-project="work"]');
    expect([...inbox.querySelectorAll('button')].map((b) => b.textContent)).toEqual(['Edit']);
    expect([...work.querySelectorAll('button')].map((b) => b.textContent)).toEqual(['Edit', 'Disable']);
    expect(host.querySelector('#notes-projects-disabled summary').textContent).toBe('Disabled (1)');
    act(() => root.unmount());
  });
});
