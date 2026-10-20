// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const calls = [];
let failNext = '';

function reply(name, args) {
  calls.push([name, ...args]);
  if (failNext) {
    const error = failNext;
    failNext = '';
    return { error };
  }
  return { ok: true };
}

vi.mock('../../frontend/src/host/api.ts', () => ({
  createWorkspace: async (...a) => reply('create', a),
  renameWorkspace: async (...a) => reply('rename', a),
  setWorkspaceViewMode: async (...a) => reply('view', a),
  setWorkspaceDescription: async (...a) => reply('description', a),
  disableWorkspace: async (...a) => reply('disable', a),
  enableWorkspace: async (...a) => reply('enable', a),
  fetchDisabledWorkspaces: async () => ({ workspaces: [{ workspace_id: 'old', workspace_name: 'Old' }] }),
}));
vi.mock('../../frontend/src/notes/commands/workspace.ts', () => ({ loadWorkspaces: async () => {} }));
vi.mock('../../frontend/src/notes/commands/workspace-switch.ts', () => ({ applyViewModeChange: () => {} }));

const { state, notifyState } = await import('../../frontend/src/host/state.ts');
const commands = await import('../../frontend/src/app-shell/commands/settings/workspaces.ts');
const { workspacesPanelStore, emptyWorkspacesPanelSnap } = await import(
  '../../frontend/src/app-shell/state/settings/workspaces-panel.ts'
);
const { NotesWorkspacesEditHost } = await import(
  '../../frontend/src/app-shell/ui/settings/notes-workspaces-edit.tsx'
);
const { NotesSettingsPanel } = await import('../../frontend/src/app-shell/ui/settings/notes-panel.tsx');

const snap = () => workspacesPanelStore.getSnapshot();

describe('Workspace editor commands', () => {
  beforeEach(() => {
    calls.length = 0;
    failNext = '';
    workspacesPanelStore.set(emptyWorkspacesPanelSnap());
    state.workspaces.current = 'default';
    state.workspaces.list = [
      { workspace_id: 'default', workspace_name: 'Default', view_mode: 'directory', is_default: true, description: '' },
      { workspace_id: 'ws2', workspace_name: 'Wiki', view_mode: 'timeline', is_default: false, description: 'Team wiki' },
    ];
  });

  it('opens with the current name and view in the draft, and closes clean', () => {
    commands.openWorkspaceEditor('ws2');
    expect(snap()).toMatchObject({
      editorOpen: true,
      selectedId: 'ws2',
      draftName: 'Wiki',
      draftViewMode: 'timeline',
      draftDescription: 'Team wiki',
    });
    commands.closeWorkspaceEditor();
    expect(snap()).toMatchObject({ editorOpen: false, selectedId: '', confirmDisable: false });
  });

  it('saves only what changed, then closes', async () => {
    commands.openWorkspaceEditor('ws2');
    commands.setWorkspaceDraft('Wiki 2', 'timeline');
    expect(await commands.saveWorkspaceEditor()).toBe(true);
    expect(calls).toEqual([['rename', 'ws2', 'Wiki 2']]);
    expect(snap().editorOpen).toBe(false);

    calls.length = 0;
    commands.openWorkspaceEditor('ws2');
    commands.setWorkspaceDraft('Wiki', 'directory');
    await commands.saveWorkspaceEditor();
    expect(calls).toEqual([['view', 'ws2', 'directory']]);

    calls.length = 0;
    commands.openWorkspaceEditor('ws2');
    commands.setWorkspaceDraftDescription('Daily notes');
    await commands.saveWorkspaceEditor();
    expect(calls).toEqual([['description', 'ws2', 'Daily notes']]);

    calls.length = 0;
    commands.openWorkspaceEditor('ws2');
    commands.setWorkspaceDraftDescription('   ');
    await commands.saveWorkspaceEditor();
    expect(calls).toEqual([['description', 'ws2', '']]);
  });

  it('keeps the dialog open and shows the refusal when the host rejects', async () => {
    commands.openWorkspaceEditor('ws2');
    commands.setWorkspaceDraft('Taken', 'timeline');
    failNext = 'name already used';
    expect(await commands.saveWorkspaceEditor()).toBe(false);
    expect(snap().editorOpen).toBe(true);
    expect(snap().error).toBe('name already used');
  });

  it('never asks to disable the built-in workspace, and disables the other after a confirm step', async () => {
    commands.openWorkspaceEditor('default');
    commands.askWorkspaceDisable();
    expect(snap().confirmDisable).toBe(false);

    commands.openWorkspaceEditor('ws2');
    commands.askWorkspaceDisable();
    expect(snap().confirmDisable).toBe(true);
    commands.cancelWorkspaceDisable();
    expect(snap().confirmDisable).toBe(false);
    expect(calls).toEqual([]);

    commands.askWorkspaceDisable();
    expect(await commands.confirmWorkspaceDisable()).toBe(true);
    expect(calls).toEqual([['disable', 'ws2']]);
    expect(snap().editorOpen).toBe(false);
  });
});

describe('Workspaces settings UI', () => {
  let host;
  let root;

  function mount(component) {
    host = document.createElement('div');
    document.body.appendChild(host);
    root = createRoot(host);
    act(() => root.render(createElement(component)));
  }

  beforeEach(async () => {
    calls.length = 0;
    workspacesPanelStore.set(emptyWorkspacesPanelSnap());
    state.workspaces.list = [
      { workspace_id: 'default', workspace_name: 'Default', view_mode: 'directory', is_default: true, description: '' },
      { workspace_id: 'ws2', workspace_name: 'Wiki', view_mode: 'timeline', is_default: false, description: 'Team wiki' },
    ];
    await commands.loadDisabledWorkspaces();
  });

  it('lists whole-row buttons and walks Edit → Disable → Back → Save', () => {
    mount(NotesWorkspacesEditHost);
    const rows = host.querySelectorAll('button.notes-cat-item[data-workspace]');
    expect([...rows].map((r) => r.dataset.workspace)).toEqual(['default', 'ws2']);
    expect(rows[1].querySelector('.notes-cat-item-action').textContent).toBe('Edit');
    expect(rows[0].querySelector('.notes-cat-item-desc').textContent).toBe('Directory view');
    expect(rows[1].querySelector('.notes-cat-item-desc').textContent).toBe('Timeline view');
    expect(rows[0].getAttribute('title')).toBeNull();
    expect(rows[1].getAttribute('title')).toBe('Team wiki');
    expect(host.querySelector('#notes-ws-edit-description')).toBeNull();
    expect(host.querySelector('#notes-ws-edit-dialog').classList.contains('open')).toBe(false);

    act(() => rows[1].click());
    expect(host.querySelector('#notes-ws-edit-dialog').classList.contains('open')).toBe(true);
    expect(host.querySelector('#notes-ws-edit-dialog-title').textContent).toBe('Edit workspace');
    expect(host.querySelector('#notes-ws-edit-description').value).toBe('Team wiki');
    const labels = () => [...host.querySelectorAll('.notes-cat-edit-actions button')].map((b) => b.textContent);
    expect(labels()).toEqual(['Disable', 'Cancel', 'Save']);

    act(() => host.querySelector('.notes-cat-edit-actions .sediment-kb-delete-btn').click());
    expect(host.querySelector('#notes-ws-edit-dialog-title').textContent).toBe('Disable workspace');
    expect(labels()).toEqual(['Back', 'Disable workspace']);
    act(() => host.querySelector('.notes-cat-edit-actions .btn-settings-ghost').click());
    expect(labels()).toEqual(['Disable', 'Cancel', 'Save']);

    act(() => rows[0].click());
    expect(labels()).toEqual(['Cancel', 'Save']);
    act(() => root.unmount());
  });

  it('keeps Disabled collapsed below the list and enables from its row', async () => {
    mount(NotesWorkspacesEditHost);
    const details = host.querySelector('details#notes-workspaces-disabled');
    expect(details.open).toBe(false);
    expect(details.querySelector('summary').textContent).toBe('Disabled (1)');
    await act(async () => details.querySelector('button[data-workspace-disabled="old"]').click());
    expect(calls).toEqual([['enable', 'old']]);
    act(() => root.unmount());
  });

  it('the Workspaces page has Directory / Add / Edit tabs and a read-only root', () => {
    workspacesPanelStore.set({ ...snap(), root: '/data/spark' });
    mount(NotesSettingsPanel);
    expect([...host.querySelectorAll('.settings-tab')].map((t) => t.dataset.tab)).toEqual(['directory', 'add', 'edit']);
    const input = host.querySelector('#workspaces-root-path');
    expect(input.readOnly).toBe(true);
    expect(input.value).toBe('/data/spark');
    expect(host.querySelector('#settings-tab-notes-directory').classList.contains('active')).toBe(true);
    act(() => root.unmount());
  });
});
