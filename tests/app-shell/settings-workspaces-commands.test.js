// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';

const calls = [];
let disabledRows = [];
let failNext = '';

vi.mock('../../frontend/src/host/api.ts', () => ({
  createWorkspace: async (...a) => reply('create', a),
  renameWorkspace: async (...a) => reply('rename', a),
  setWorkspaceViewMode: async (...a) => reply('view', a),
  setWorkspaceDescription: async (...a) => reply('description', a),
  disableWorkspace: async (...a) => reply('disable', a),
  enableWorkspace: async (...a) => reply('enable', a),
  fetchDisabledWorkspaces: async () => ({ workspaces: disabledRows }),
}));

const loadWorkspaces = vi.fn(async () => {});
vi.mock('../../frontend/src/notes/commands/workspace.ts', () => ({ loadWorkspaces: () => loadWorkspaces() }));
const applyViewModeChange = vi.fn();
vi.mock('../../frontend/src/notes/commands/workspace-switch.ts', () => ({
  applyViewModeChange: () => applyViewModeChange(),
}));

function reply(name, args) {
  calls.push([name, ...args]);
  if (failNext) {
    const error = failNext;
    failNext = '';
    return { error };
  }
  return { ok: true };
}

const { state } = await import('../../frontend/src/host/state.ts');
const commands = await import('../../frontend/src/app-shell/commands/settings/workspaces.ts');
const { workspacesPanelStore, emptyWorkspacesPanelSnap } = await import(
  '../../frontend/src/app-shell/state/settings/workspaces-panel.ts'
);

describe('Workspaces settings commands', () => {
  beforeEach(() => {
    calls.length = 0;
    disabledRows = [];
    failNext = '';
    loadWorkspaces.mockClear();
    applyViewModeChange.mockClear();
    workspacesPanelStore.set(emptyWorkspacesPanelSnap());
    state.workspaces.current = 'default';
  });

  it('disables a workspace, reloads the registry and fills the Disabled list', async () => {
    disabledRows = [{ workspace_id: 'ws-a', workspace_name: 'Research' }];
    expect(await commands.disableWorkspace('ws-a')).toBe(true);
    expect(calls).toEqual([['disable', 'ws-a']]);
    expect(loadWorkspaces).toHaveBeenCalledTimes(1);
    expect(workspacesPanelStore.getSnapshot().disabled).toEqual([{ id: 'ws-a', name: 'Research', sortIndex: 0 }]);
    expect(workspacesPanelStore.getSnapshot().busy).toBe(false);
  });

  it('reloads the page index when the shown workspace is no longer enabled', async () => {
    state.workspaces.current = 'ws-a';
    const reloads = vi.fn();
    document.addEventListener('cta:reload', reloads);
    loadWorkspaces.mockImplementationOnce(async () => {
      state.workspaces.current = 'default';
    });
    await commands.disableWorkspace('ws-a');
    document.removeEventListener('cta:reload', reloads);
    expect(reloads).toHaveBeenCalledTimes(1);
  });

  it('keeps the error and skips reloads when the host refuses', async () => {
    failNext = 'Default workspace cannot be disabled';
    expect(await commands.disableWorkspace('default')).toBe(false);
    expect(workspacesPanelStore.getSnapshot().error).toBe('Default workspace cannot be disabled');
    expect(loadWorkspaces).not.toHaveBeenCalled();
    expect(workspacesPanelStore.getSnapshot().busy).toBe(false);
  });

  it('enables a disabled workspace again', async () => {
    expect(await commands.enableWorkspace('ws-a')).toBe(true);
    expect(calls).toEqual([['enable', 'ws-a']]);
    expect(workspacesPanelStore.getSnapshot().disabled).toEqual([]);
  });

  it('regroups the page only when the changed workspace is the shown one', async () => {
    await commands.changeWorkspaceViewMode('ws-a', 'timeline');
    expect(applyViewModeChange).not.toHaveBeenCalled();
    state.workspaces.current = 'ws-a';
    await commands.changeWorkspaceViewMode('ws-a', 'directory');
    expect(applyViewModeChange).toHaveBeenCalledTimes(1);
    expect(calls).toEqual([['view', 'ws-a', 'timeline'], ['view', 'ws-a', 'directory']]);
  });

  it('creates a workspace with a description only when one is given', async () => {
    await commands.addWorkspace('Plain', 'directory');
    await commands.addWorkspace('Research', 'timeline', '', 'Daily notes');
    expect(calls).toEqual([
      ['create', 'Plain', 'directory'],
      ['create', 'Research', 'timeline', undefined, 'Daily notes'],
    ]);
  });
});
