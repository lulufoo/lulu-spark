// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const order = [];
let resolveShow;
const projectsByWorkspace = {
  default: [{ folder: 'inbox', title: 'Inbox' }, { folder: 'work', title: 'Work' }],
  knowledge: [
    { folder: 'a', title: 'A' },
    { folder: 'b', title: 'B' },
    { folder: 'c', title: 'C' },
    { folder: 'd', title: 'D' },
  ],
  empty: [],
};

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchProjects: async (id) => {
    if (id === 'broken') throw new Error('boom');
    return { projects: projectsByWorkspace[id] ?? [] };
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
const { getHomeState, resetHomeState } = await import('../../frontend/src/home/state/store.ts');
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
      list: rows.map((r) => ({ id: r.id, name: r.name, view_mode: 'directory', is_default: r.isDefault })),
    };
  });

  it('loads the projects of every workspace and tolerates one that fails', async () => {
    state.workspaces.list.push({ id: 'broken', name: 'Broken', view_mode: 'directory', is_default: false });
    await commands.refreshWorkspaceProjects();
    const loaded = getHomeState().workspaceProjects;
    expect(loaded.default.map((p) => p.folder)).toEqual(['inbox', 'work']);
    expect(loaded.knowledge).toHaveLength(4);
    expect(loaded.broken).toEqual([]);
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
  function render(props) {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    act(() =>
      root.render(
        createElement(HomeWorkspaceNav, {
          workspaces: rows,
          projects: projectsByWorkspace,
          unreadDocs: false,
          onOpenWorkspace: vi.fn(),
          onOpenProject: vi.fn(),
          onCreateDocument: vi.fn(),
          ...props,
        }),
      ),
    );
    return host;
  }

  it('lists at most three projects per workspace and a ... row only beyond three', () => {
    const host = render();
    const projectsOf = (id) =>
      [...host.querySelectorAll(`[data-home-workspace="${id}"] .home-nav-project:not(.home-nav-more)`)].map(
        (b) => b.dataset.homeProject,
      );
    expect(projectsOf('default')).toEqual(['inbox', 'work']);
    expect(projectsOf('knowledge')).toEqual(['a', 'b', 'c']);
    expect(projectsOf('empty')).toEqual([]);
    expect(host.querySelector('[data-home-more="knowledge"]').textContent).toBe('...');
    expect(host.querySelector('[data-home-more="default"]')).toBeNull();
    expect(host.querySelector('[data-home-more="empty"]')).toBeNull();
  });

  it('keeps one shortcut per workspace, the entry keys and the + beside Default only', () => {
    const host = render({ unreadDocs: true });
    expect([...host.querySelectorAll('.home-desktop-shortcut')].map((b) => b.dataset.homeEntry)).toEqual([
      'spark',
      'knowledge',
      'workspace:empty',
    ]);
    expect(host.querySelectorAll('[data-role="create-note"]')).toHaveLength(1);
    expect(host.querySelector('[data-home-entry="spark"]').dataset.homeUnread).toBe('true');
    expect(host.querySelector('[data-home-entry="knowledge"]').dataset.homeUnread).toBeUndefined();
  });

  it('routes clicks: project selects it, workspace and ... select nothing', () => {
    const onOpenWorkspace = vi.fn();
    const onOpenProject = vi.fn();
    const host = render({ onOpenWorkspace, onOpenProject });
    host.querySelector('[data-home-project="b"]').click();
    expect(onOpenProject).toHaveBeenCalledWith('knowledge', 'b');
    host.querySelector('[data-home-entry="knowledge"]').click();
    host.querySelector('[data-home-more="knowledge"]').click();
    expect(onOpenWorkspace).toHaveBeenCalledTimes(2);
    expect(onOpenWorkspace).toHaveBeenCalledWith('knowledge');
    expect(onOpenProject).toHaveBeenCalledTimes(1);
  });
});
