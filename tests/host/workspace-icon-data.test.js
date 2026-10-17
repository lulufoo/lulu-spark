// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';

let workspaces = [];
let projects = [];

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchWorkspaces: async () => ({ workspaces }),
  fetchProjects: async () => ({ projects }),
}));
vi.mock('../../frontend/src/notes/commands/sidebar.ts', () => ({ selectProject: () => {}, selectDefaultProject: () => {} }));
vi.mock('../../frontend/src/notes/commands/workspace-switch.ts', () => ({
  switchWorkspace: () => {},
  showWorkspace: async () => {},
}));
vi.mock('../../frontend/src/notes/commands/viewer-state.ts', () => ({ restoreOpenDoc: async () => {} }));

const { state } = await import('../../frontend/src/host/state.ts');
const { loadWorkspaces } = await import('../../frontend/src/notes/commands/workspace.ts');
const { refreshWorkspaceProjects } = await import('../../frontend/src/home/commands/workspace-nav.ts');
const { getHomeState } = await import('../../frontend/src/home/state/store.ts');

describe('icon fields in the data layer', () => {
  it('keeps the icon of a workspace and reads a missing one as empty', async () => {
    workspaces = [
      { id: 'default', name: 'Default', view_mode: 'directory', is_default: true, icon: '03ab12' },
      { id: 'old', name: 'Old', view_mode: 'timeline', is_default: false },
      { id: 'odd', name: 'Odd', view_mode: 'timeline', is_default: false, icon: 7 },
    ];
    await loadWorkspaces();
    expect(state.workspaces.list.map((w) => w.icon)).toEqual(['03ab12', '', '']);
  });

  it('keeps the icon of a project in the Home sidebar and reads a missing one as empty', async () => {
    state.workspaces.list = [{ id: 'default', name: 'Default', view_mode: 'directory', is_default: true, icon: '' }];
    projects = [
      { id: 'plans', title: 'Plans', icon: '01aa00' },
      { id: 'inbox', title: 'Inbox' },
    ];
    await refreshWorkspaceProjects();
    expect(getHomeState().workspaceProjects.default).toEqual([
      { id: 'plans', title: 'Plans', icon: '01aa00' },
      { id: 'inbox', title: 'Inbox', icon: '' },
    ]);
  });
});
