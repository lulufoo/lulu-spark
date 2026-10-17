// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

let projectsByWorkspace = {};

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchProjects: async (id) => ({ projects: projectsByWorkspace[id] ?? [] }),
  invoke: async () => ({ state: 'unbound' }),
  createChannel: async (onmessage) => ({ onmessage }),
}));
vi.mock('../../frontend/src/notes/commands/sidebar.ts', () => ({ selectProject() {}, selectDefaultProject() {} }));
vi.mock('../../frontend/src/notes/commands/workspace-switch.ts', () => ({
  switchWorkspace() {},
  showWorkspace: async () => {},
}));
vi.mock('../../frontend/src/notes/commands/viewer-state.ts', () => ({ restoreOpenDoc: async () => {} }));

const { state } = await import('../../frontend/src/host/state.ts');
const { HomeWorkspaceNav } = await import('../../frontend/src/home/ui/workspace-nav.tsx');
const { startHomeHub, stopHomeHub } = await import('../../frontend/src/home/commands/hub.ts');
const { getHomeState, resetHomeState } = await import('../../frontend/src/home/state/store.ts');
const { announceRegistryChange } = await import('../../frontend/src/shared/registry-events.ts');
const { ICONS, DEFAULT_PROJECT_ICON_ID } = await import('../../frontend/src/shared/icons/index.ts');

const ICON_WS = ICONS.find((i) => i.category === '02').id;
const ICON_P = ICONS.find((i) => i.category === '04').id;
const ICON_P2 = ICONS.find((i) => i.category === '05').id;

const rows = (icons = {}) => [
  { id: 'default', name: 'Default', isDefault: true, icon: icons.default ?? '' },
  { id: 'knowledge', name: 'Knowledge', isDefault: false, icon: icons.knowledge ?? '' },
  { id: 'wiki', name: 'Wiki', isDefault: false, icon: icons.wiki ?? '' },
];

describe('HomeWorkspaceNav icons', () => {
  let hosts;
  let listHeight;

  beforeEach(() => {
    hosts = [];
    listHeight = 20 * 32;
    globalThis.ResizeObserver = class {
      observe() {}
      disconnect() {}
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

  function render(workspaces, projects) {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    hosts.push({ root, host });
    act(() =>
      root.render(
        createElement(HomeWorkspaceNav, {
          workspaces,
          projects,
          unreadDocs: false,
          onOpenWorkspace() {},
          onOpenProject() {},
          onCreateDocument() {},
        }),
      ),
    );
    return host;
  }

  const entry = (host, key) => host.querySelector(`[data-home-entry="${key}"]`);
  const project = (host, folder) => host.querySelector(`[data-home-project="${folder}"]`);

  it('draws the configured icon of a workspace and of a project', () => {
    const host = render(rows({ default: ICON_WS }), {
      default: [{ folder: 'plans', title: 'Plans', icon: ICON_P }],
    });
    expect(entry(host, 'spark').querySelector(`svg[data-icon-id="${ICON_WS}"]`)).not.toBeNull();
    expect(entry(host, 'spark').querySelector('[data-home-icon="notes"]')).toBeNull();
    expect(project(host, 'plans').querySelector(`svg[data-icon-id="${ICON_P}"]`)).not.toBeNull();
  });

  it('keeps today’s icons for workspaces without one: notes, and a book for Knowledge', () => {
    const host = render(rows(), {});
    expect(entry(host, 'spark').querySelector('[data-home-icon="notes"]')).not.toBeNull();
    expect(entry(host, 'knowledge').querySelector('[data-home-icon="knowledge"]')).not.toBeNull();
    expect(entry(host, 'workspace:wiki').querySelector('[data-home-icon="notes"]')).not.toBeNull();
    expect(host.querySelector('[data-icon-id]')).toBeNull();
  });

  it('gives a project without an icon the Folder icon', () => {
    const host = render(rows(), { default: [{ folder: 'inbox', title: 'Inbox', icon: '' }] });
    expect(project(host, 'inbox').querySelector(`svg[data-icon-id="${DEFAULT_PROJECT_ICON_ID}"]`)).not.toBeNull();
  });

  it('falls back to the defaults for a well-formed id the module does not know', () => {
    const host = render(rows({ default: '00zzzz' }), {
      default: [{ folder: 'inbox', title: 'Inbox', icon: '00zzzz' }],
    });
    expect(entry(host, 'spark').querySelector('[data-home-icon="notes"]')).not.toBeNull();
    expect(project(host, 'inbox').querySelector(`svg[data-icon-id="${DEFAULT_PROJECT_ICON_ID}"]`)).not.toBeNull();
  });

  it('draws icons in the current colour, with no colour of their own', () => {
    const host = render(rows({ default: ICON_WS }), { default: [{ folder: 'p', title: 'P', icon: ICON_P }] });
    for (const svg of host.querySelectorAll('svg[data-icon-id]')) {
      expect(svg.getAttribute('stroke')).toBe('currentColor');
    }
  });

  it('puts no icon on the ... row, and icons do not change which rows are shown', () => {
    listHeight = 7 * 32;
    const host = render(rows(), {
      default: [
        { folder: 'inbox', title: 'Inbox', icon: ICON_P },
        { folder: 'work', title: 'Work', icon: ICON_P2 },
      ],
      knowledge: ['a', 'b', 'c', 'd'].map((folder) => ({ folder, title: folder, icon: ICON_P })),
      wiki: [],
    });
    const more = host.querySelector('[data-home-more="knowledge"]');
    expect(more).not.toBeNull();
    expect(more.querySelector('svg')).toBeNull();
    expect(more.querySelector('.home-nav-project-icon')).toBeNull();
    const shown = [...host.querySelectorAll('.home-nav-project:not(.home-nav-more)')].map((b) => b.dataset.homeProject);
    expect(shown).toEqual(['inbox', 'work', 'a']);
    for (const button of host.querySelectorAll('.home-nav-project:not(.home-nav-more)')) {
      expect(button.querySelector('.home-nav-project-icon svg')).not.toBeNull();
    }
    const rowCount = host.querySelectorAll('.home-chat-nav-notes, .home-nav-project').length;
    expect(rowCount).toBeLessThanOrEqual(7);
  });

  it('keeps every row at the fixed row height and indents projects under the workspace', () => {
    const css = readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');
    expect(css).toMatch(/\.home-chat-nav-item \{[\s\S]*?height: var\(--home-nav-row-h\);/);
    const projectRule = css.match(/\.home-chat-nav \.home-nav-project \{[^}]*\}/)[0];
    expect(projectRule).toMatch(/padding: 0 8px 0 24px;/);
    expect(projectRule).not.toMatch(/\bheight:/);
    expect(css).toMatch(/\.home-chat-nav \.home-nav-more \{ padding-left: 48px;/);
    expect(css).toMatch(/\.home-nav-project-icon \{[^}]*width: 16px;/);
  });

  it('leaves Read Later on its own fixed icon, which cannot be configured', () => {
    const page = readFileSync(join(repoRoot, 'frontend/src/home/page.tsx'), 'utf8');
    expect(page).toMatch(/<HomeNavIcon name="read-later" \/>/);
    const nav = readFileSync(join(repoRoot, 'frontend/src/home/ui/workspace-nav.tsx'), 'utf8');
    expect(nav).not.toMatch(/read-later/);
  });
});

describe('Home sidebar after a settings change', () => {
  afterEach(() => {
    stopHomeHub();
    resetHomeState();
  });

  it('re-reads the projects, with their new icons, when the registry changes', async () => {
    state.workspaces.list = [
      { id: 'default', name: 'Default', view_mode: 'directory', is_default: true, icon: '' },
    ];
    projectsByWorkspace = { default: [{ folder: 'plans', title: 'Plans', icon: '' }] };
    startHomeHub();
    announceRegistryChange();
    await vi.waitFor(() => expect(getHomeState().workspaceProjects.default?.[0]?.icon).toBe(''));

    projectsByWorkspace = { default: [{ folder: 'plans', title: 'Plans', icon: ICON_P }] };
    announceRegistryChange();
    await vi.waitFor(() => expect(getHomeState().workspaceProjects.default[0].icon).toBe(ICON_P));
  });

  it('passes the workspace icon from the registry list to the sidebar rows', () => {
    const page = readFileSync(join(repoRoot, 'frontend/src/home/page.tsx'), 'utf8');
    expect(page).toMatch(/icon: w\.icon/);
  });
});
