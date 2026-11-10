// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const order = [];
let resolveShow;
const projectsByWorkspace = {
  default: [{ id: 'inbox', title: 'Inbox' }, { id: 'work', title: 'Work' }],
  knowledge: [
    { id: 'a', title: 'A' },
    { id: 'b', title: 'B' },
    { id: 'c', title: 'C' },
    { id: 'd', title: 'D' },
  ],
  empty: [],
};

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchProjects: async (id) => {
    if (id === 'broken') throw new Error('boom');
    return {
      projects: (projectsByWorkspace[id] ?? []).map((p) => ({ project_id: p.id, project_name: p.title })),
    };
  },
}));
vi.mock('../../frontend/src/notes/commands/sidebar.ts', () => ({
  selectProject: (folder) => order.push(['select', folder]),
  selectDefaultProject: () => order.push(['clear']),
}));
vi.mock('../../frontend/src/notes/commands/workspace-switch.ts', () => ({
  switchWorkspace: (id) => order.push(['switch', id]),
  showWorkspace: (id) => {
    order.push(['show', id]);
    return new Promise((resolve) => {
      resolveShow = () => {
        order.push(['shown', id]);
        resolve();
      };
    });
  },
}));

const { state } = await import('../../frontend/src/host/state.ts');
const commands = await import('../../frontend/src/home/commands/workspace-nav.ts');
const { getHomeState, resetHomeState, setHomeState, useHomeState } = await import('../../frontend/src/home/state/store.ts');
const { HomeWorkspaceNav } = await import('../../frontend/src/home/ui/workspace-nav.tsx');

const rows = [
  { id: 'default', name: 'Default', isDefault: true },
  { id: 'knowledge', name: 'Knowledge', isDefault: false },
  { id: 'empty', name: 'Empty', isDefault: false },
];

describe('Home workspace nav commands', () => {
  beforeEach(() => {
    order.length = 0;
    resetHomeState();
    state.workspaces = {
      current: 'default',
      list: rows.map((r) => ({ workspace_id: r.id, workspace_name: r.name, view_mode: 'directory', is_default: r.isDefault })),
    };
  });

  it('loads the projects of every workspace and tolerates one that fails', async () => {
    state.workspaces.list.push({ workspace_id: 'broken', workspace_name: 'Broken', view_mode: 'directory', is_default: false });
    await commands.refreshWorkspaceProjects();
    const loaded = getHomeState().workspaceProjects;
    expect(loaded.default.map((p) => p.id)).toEqual(['inbox', 'work']);
    expect(loaded.knowledge).toHaveLength(4);
    expect(loaded.broken).toEqual([]);
  });

  it('toggles known workspaces independently, without navigation', () => {
    commands.toggleWorkspaceProjects('default');
    commands.toggleWorkspaceProjects('knowledge');
    expect(getHomeState().expandedWorkspaceIds).toEqual(['default', 'knowledge']);
    commands.toggleWorkspaceProjects('default');
    commands.toggleWorkspaceProjects('missing');
    expect(getHomeState().expandedWorkspaceIds).toEqual(['knowledge']);
    expect(order).toEqual([]);
  });

  it('prunes removed groups on refresh and preserves valid expansion', async () => {
    setHomeState((prev) => ({ ...prev, expandedWorkspaceIds: ['default', 'knowledge'] }));
    state.workspaces.list = state.workspaces.list.filter((row) => row.workspace_id !== 'knowledge');
    await commands.refreshWorkspaceProjects();
    expect(getHomeState().expandedWorkspaceIds).toEqual(['default']);
    expect(getHomeState().workspaceProjects.knowledge).toBeUndefined();
  });

  it('opening another workspace switches and selects nothing, then navigates', () => {
    const navigate = vi.fn();
    commands.openWorkspace('knowledge', navigate);
    expect(order).toEqual([['switch', 'knowledge']]);
    expect(navigate).toHaveBeenCalledWith('#/spark');
  });

  it('opening the shown workspace selects its default project', () => {
    const navigate = vi.fn();
    commands.openWorkspace('default', navigate);
    expect(order).toEqual([['clear']]);
    expect(navigate).toHaveBeenCalledWith('#/spark');
  });

  it('opening a project waits for the workspace index before selecting it', async () => {
    const navigate = vi.fn();
    const pending = commands.openProject('knowledge', 'b', navigate);
    expect(order).toEqual([['show', 'knowledge']]);
    expect(navigate).not.toHaveBeenCalled();
    resolveShow();
    await pending;
    expect(order).toEqual([['show', 'knowledge'], ['shown', 'knowledge'], ['select', 'b']]);
    expect(navigate).toHaveBeenCalledWith('#/spark');
  });
});

describe('HomeWorkspaceNav', () => {
  const hosts = [];
  beforeEach(() => {
    resetHomeState();
    state.workspaces.list = rows.map((row) => ({ workspace_id: row.id }));
  });
  afterEach(() => {
    for (const { root, host } of hosts.splice(0)) {
      act(() => root.unmount());
      host.remove();
    }
  });

  function render(props = {}) {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    hosts.push({ root, host });
    function Harness({ next }) {
      const home = useHomeState();
      return createElement(HomeWorkspaceNav, {
      workspaces: rows,
      projects: projectsByWorkspace,
      unreadDocs: true,
      expandedWorkspaceIds: home.expandedWorkspaceIds,
      onToggleWorkspace: commands.toggleWorkspaceProjects,
      onOpenWorkspace: vi.fn(),
      onOpenProject: vi.fn(),
      ...props, ...next,
      });
    }
    const paint = (next = {}) => act(() => root.render(createElement(Harness, { next })));
    paint();
    return { host, paint };
  }

  it('starts with all workspaces collapsed and keeps every workspace title', () => {
    const { host } = render();
    expect(host.querySelectorAll('.home-nav-project')).toHaveLength(0);
    expect(host.querySelectorAll('.home-desktop-shortcut')).toHaveLength(3);
    expect(host.querySelector('[data-home-entry="spark"]').dataset.homeUnread).toBe('true');
  });

  it('shows all projects in every explicitly expanded workspace, even in a short list', () => {
    const { host, paint } = render({ expandedWorkspaceIds: ['knowledge', 'default'] });
    host.style.height = '32px';
    expect(host.querySelectorAll('.home-nav-project')).toHaveLength(6);
    expect([...host.querySelectorAll('[data-home-workspace="knowledge"] .home-nav-project')].map((el) => el.dataset.homeProject)).toEqual(['a', 'b', 'c', 'd']);
    paint({ expandedWorkspaceIds: ['knowledge'] });
    expect(host.querySelectorAll('.home-nav-project')).toHaveLength(4);
  });

  it('updates projects and workspace names without changing the expansion selection', () => {
    const { host, paint } = render({ expandedWorkspaceIds: ['knowledge'] });
    paint({ workspaces: rows.map((row) => ({ ...row, name: 'Renamed ' + row.name })), projects: { knowledge: [{ id: 'new', title: 'New project' }] } });
    expect(host.querySelector('[data-home-entry="knowledge"]').textContent).toContain('Renamed Knowledge');
    expect(host.querySelector('[data-home-project="new"]')).not.toBeNull();
    expect(host.querySelector('[data-home-project="a"]')).toBeNull();
  });

  it('toggles the name region without navigation and keeps the ellipsis separate', () => {
    const onOpenWorkspace = vi.fn();
    const { host } = render({ onOpenWorkspace });
    const name = host.querySelector('[data-home-entry="knowledge"]');
    const enter = host.querySelector('[data-home-open-workspace="knowledge"]');
    expect(enter.textContent.trim()).toBe('…');
    expect(enter.closest('button button')).toBeNull();
    expect(name.getAttribute('aria-expanded')).toBe('false');
    act(() => name.click());
    expect(name.getAttribute('aria-expanded')).toBe('true');
    expect(host.querySelector('[id="' + name.getAttribute('aria-controls') + '"]').querySelectorAll('button')).toHaveLength(4);
    expect(onOpenWorkspace).not.toHaveBeenCalled();
    act(() => enter.click());
    expect(onOpenWorkspace).toHaveBeenCalledExactlyOnceWith('knowledge');
    expect(name.getAttribute('aria-expanded')).toBe('true');
    act(() => name.click());
    expect(name.getAttribute('aria-expanded')).toBe('false');
    expect(host.querySelector('[data-home-project="a"]')).toBeNull();
    act(() => enter.click());
    expect(onOpenWorkspace).toHaveBeenCalledTimes(2);
    expect(name.getAttribute('aria-expanded')).toBe('false');
  });

  it('can expand multiple workspaces and keeps the plain ellipsis visible on every row', () => {
    const { host } = render();
    expect(host.querySelectorAll('[data-home-open-workspace]')).toHaveLength(rows.length);
    act(() => host.querySelector('[data-home-entry="spark"]').click());
    act(() => host.querySelector('[data-home-entry="knowledge"]').click());
    expect(host.querySelectorAll('.home-nav-project')).toHaveLength(6);
    expect(host.querySelector('[data-home-open-workspace="empty"]')).not.toBeNull();
  });

  it('routes an expanded project through the existing command callback', () => {
    const onOpenProject = vi.fn();
    const { host } = render({ expandedWorkspaceIds: ['knowledge'], onOpenProject });
    act(() => host.querySelector('[data-home-project="d"]').click());
    expect(onOpenProject).toHaveBeenCalledWith('knowledge', 'd');
  });
});
