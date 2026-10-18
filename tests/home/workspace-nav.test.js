// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

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
  const ROW_H = 32;
  const GAP_H = 8;
  const GAPS = rows.length - 1;
  let listHeight;
  let observers;
  let hosts;

  beforeEach(() => {
    listHeight = 20 * ROW_H;
    observers = [];
    hosts = [];
    globalThis.ResizeObserver = class {
      constructor(callback) {
        this.callback = callback;
        observers.push(this);
      }
      observe() {}
      disconnect() {
        observers = observers.filter((o) => o !== this);
      }
    };
    Object.defineProperty(HTMLElement.prototype, 'clientHeight', {
      configurable: true,
      get() {
        return this.dataset?.homeNavList ? listHeight : 0;
      },
    });
  });

  afterEach(() => {
    for (const { root, host } of hosts) {
      act(() => root.unmount());
      host.remove();
    }
    delete globalThis.ResizeObserver;
    delete HTMLElement.prototype.clientHeight;
  });

  function render(props) {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    hosts.push({ root, host });
    act(() =>
      root.render(
        createElement(HomeWorkspaceNav, {
          workspaces: rows,
          projects: projectsByWorkspace,
          unreadDocs: false,
          onOpenWorkspace: vi.fn(),
          onOpenProject: vi.fn(),
          ...props,
        }),
      ),
    );
    return host;
  }

  /** A list tall enough for `rowCount` whole rows plus the gaps between the workspaces. */
  function resizeTo(rowCount) {
    listHeight = rowCount * ROW_H + GAPS * GAP_H;
    act(() => observers.forEach((o) => o.callback([])));
  }

  const projectsOf = (host, id) =>
    [...host.querySelectorAll(`[data-home-workspace="${id}"] .home-nav-project`)].map(
      (b) => b.dataset.homeProject,
    );

  it('shows every project and no "…" mark when all of them fit', () => {
    const host = render();
    expect(projectsOf(host, 'default')).toEqual(['inbox', 'work']);
    expect(projectsOf(host, 'knowledge')).toEqual(['a', 'b', 'c', 'd']);
    expect(projectsOf(host, 'empty')).toEqual([]);
    expect(host.querySelector('.home-nav-hidden-count')).toBeNull();
    expect(host.querySelector('.home-nav-workspaces').dataset.homeNavOverflow).toBe('false');
  });

  it('cuts the largest workspace off and shows the "…" mark on its title row', () => {
    // Needs are 3, 5 and 1 rows; 7 rows leave Knowledge three: title and two projects.
    const host = render();
    resizeTo(7);
    expect(projectsOf(host, 'default')).toEqual(['inbox', 'work']);
    expect(projectsOf(host, 'knowledge')).toEqual(['a', 'b']);
    const badge = host.querySelector('[data-home-entry="knowledge"] .home-nav-hidden-count');
    expect(badge.textContent).toBe('…');
    expect(badge.dataset.homeHidden).toBe('knowledge');
    expect(badge.dataset.homeHiddenCount).toBe('2');
    expect(badge.getAttribute('data-tip')).toBe('2 projects collapsed');
    expect(badge.hasAttribute('title')).toBe(false);
    expect(badge.getAttribute('aria-label')).toBe('2 projects collapsed');
    expect(host.querySelectorAll('.home-nav-hidden-count')).toHaveLength(1);
    expect(host.querySelector('[data-home-more]')).toBeNull();
  });

  it('keeps a title and one project even with only two rows, the rest counted in the badge', () => {
    const host = render();
    resizeTo(6);
    expect(projectsOf(host, 'knowledge')).toEqual(['a']);
    const badge = host.querySelector('[data-home-entry="knowledge"] .home-nav-hidden-count');
    expect(badge.textContent).toBe('…');
    expect(badge.getAttribute('data-tip')).toBe('3 projects collapsed');
    expect(badge.hasAttribute('title')).toBe(false);
    expect(projectsOf(host, 'default')).toEqual(['inbox', 'work']);
  });

  it('recomputes when the list is resized', () => {
    const host = render();
    resizeTo(7);
    expect(projectsOf(host, 'knowledge')).toEqual(['a', 'b']);
    resizeTo(20);
    expect(projectsOf(host, 'knowledge')).toEqual(['a', 'b', 'c', 'd']);
    expect(host.querySelector('.home-nav-hidden-count')).toBeNull();
  });

  it('scrolls and keeps every title when even the titles do not fit', () => {
    const host = render();
    resizeTo(2);
    const list = host.querySelector('.home-nav-workspaces');
    expect(list.dataset.homeNavOverflow).toBe('true');
    expect(host.querySelectorAll('.home-desktop-shortcut')).toHaveLength(3);
    expect(host.querySelector('.home-nav-project')).toBeNull();
    expect(host.querySelector('.home-nav-hidden-count')).toBeNull();
  });

  it('counts the gaps between workspaces: one pixel short of rows + gaps cuts a row', () => {
    // Needs are 3 + 5 + 1 = 9 rows; with 2 gaps of 8px the exact height shows everything.
    const host = render();
    resizeTo(9);
    expect(host.querySelector('.home-nav-hidden-count')).toBeNull();
    expect(projectsOf(host, 'knowledge')).toEqual(['a', 'b', 'c', 'd']);
    listHeight -= 1;
    act(() => observers.forEach((o) => o.callback([])));
    expect(projectsOf(host, 'knowledge')).toHaveLength(3);
    expect(host.querySelector('[data-home-entry="knowledge"] .home-nav-hidden-count').dataset.homeHiddenCount).toBe('1');
  });

  it('puts an 8px gap above every workspace but the first, and none inside a workspace', () => {
    const css = readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');
    expect(css).toMatch(/\.home-chat-sidebar \{[\s\S]*?--home-nav-group-gap: 8px;/);
    expect(css).toMatch(/\.home-nav-workspace \+ \.home-nav-workspace \{[^}]*margin-top: var\(--home-nav-group-gap\);/);
    expect(css).not.toMatch(/(?<!\+ )\.home-nav-workspace \{[^}]*margin/);
    expect(css).not.toMatch(/\.home-chat-nav-notes \{[^}]*margin/);
    expect(css).not.toMatch(/\.home-nav-project \{[^}]*margin/);
  });

  it('never renders more rows than were measured', () => {
    const host = render();
    for (const count of [3, 4, 5, 6, 7, 8, 9, 12]) {
      resizeTo(count);
      const rowCount = host.querySelectorAll('.home-chat-nav-notes, .home-nav-project').length;
      expect(rowCount).toBeLessThanOrEqual(count);
    }
  });

  it('does not own Read Later, which lives in the top bar and not in the Workspace group', () => {
    const host = render();
    expect(host.querySelector('[data-home-entry="read-later"]')).toBeNull();
    const page = readFileSync(join(repoRoot, 'frontend/src/home/page.tsx'), 'utf8');
    expect(page.indexOf('<TopActions')).toBeLessThan(page.indexOf('<HomeWorkspaceNav'));
    expect(page).not.toMatch(/data-home-entry="read-later"/);
    const group = page.slice(page.indexOf('data-home-nav-group="workspace"'), page.indexOf('</nav>'));
    expect(group).not.toMatch(/read-later|Read Later/);
    const css = readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');
    expect(css).toMatch(/\.home-nav-workspaces \{[^}]*flex: 1 1 0;[^}]*min-height: 0;/);
    expect(css).toMatch(/--home-nav-row-h: 32px;/);
  });

  it('no longer has the fixed three-project limit', () => {
    for (const file of ['frontend/src/home/ui/workspace-nav.tsx', 'frontend/src/home/commands/workspace-nav.ts']) {
      expect(readFileSync(join(repoRoot, file), 'utf8')).not.toMatch(/HOME_PROJECT_LIMIT/);
    }
  });

  it('keeps one shortcut per workspace and the entry keys, with no + on any row', () => {
    const host = render({ unreadDocs: true });
    expect([...host.querySelectorAll('.home-desktop-shortcut')].map((b) => b.dataset.homeEntry)).toEqual([
      'spark',
      'knowledge',
      'workspace:empty',
    ]);
    expect(host.querySelector('[data-role="create-note"], .home-chat-new')).toBeNull();
    expect(host.querySelector('[data-home-entry="spark"]').dataset.homeUnread).toBe('true');
    expect(host.querySelector('[data-home-entry="knowledge"]').dataset.homeUnread).toBeUndefined();
  });

  it('routes clicks: a project selects it, the workspace row and its … mark enter the workspace', () => {
    const onOpenWorkspace = vi.fn();
    const onOpenProject = vi.fn();
    const host = render({ onOpenWorkspace, onOpenProject });
    resizeTo(8); // Knowledge shows a, b, c and a +1 badge.
    host.querySelector('[data-home-project="b"]').click();
    expect(onOpenProject).toHaveBeenCalledWith('knowledge', 'b');
    host.querySelector('[data-home-entry="knowledge"]').click();
    host.querySelector('[data-home-hidden="knowledge"]').click();
    expect(onOpenWorkspace).toHaveBeenCalledTimes(2);
    expect(onOpenWorkspace).toHaveBeenCalledWith('knowledge');
    expect(onOpenProject).toHaveBeenCalledTimes(1);
  });
});
