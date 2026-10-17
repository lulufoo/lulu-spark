// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../frontend/src/notes/commands/cards.ts', () => ({ loadTitles: vi.fn() }));
vi.mock('../../frontend/src/router/index.ts', () => ({
  parseHash: vi.fn(() => ({ name: 'spark', params: {} })),
  navigateToDateList: vi.fn(),
}));

const { notifyState, state } = await import('../../frontend/src/host/state.ts');
const {
  buildDirTree,
  dirKeyOf,
  dirTreeStore,
  findDirNode,
  nodeKeyOfEntry,
  pathKeys,
  resetDirTree,
} = await import('../../frontend/src/notes/state/dir-tree.ts');
const { applyListFilters, buildGroups, selectDate, selectDefaultProject, selectProject, selectTopic } = await import(
  '../../frontend/src/notes/commands/sidebar.ts'
);
const { applyViewModeChange } = await import('../../frontend/src/notes/commands/workspace-switch.ts');
const { DirTree } = await import('../../frontend/src/notes/ui/dir-tree.tsx');
const { entryGroupKey } = await import('../../frontend/src/notes/commands/workspace.ts');

const doc = (folder, raw_path, extra = {}) => ({ folder, raw_path, ...extra });

const data = {
  a: doc('android', 'arch/build/builder/detekt.md', { created_at: '202604011200' }),
  b: doc('android', 'arch/build/builder/lint.md', { created_at: '202604021200' }),
  c: doc('android', 'arch/intro.md'),
  d: doc('android', 'readme.md'),
  e: doc('ai', 'topic-x/one.md', { created_at: '202605011200' }),
  f: doc('ai', 'topic-x/two.md', { created_at: '202605021200' }),
  g: doc('ai', ''),
};
const titles = { android: 'Android', ai: 'AI', empty: 'Empty project' };

function setView(mode) {
  state.workspaces = {
    current: 'default',
    list: [{ id: 'default', name: 'Default', view_mode: mode, is_default: true }],
  };
}

function load() {
  state.index.data = data;
  state.index.topicTitles = titles;
  state.index.groupedByDate = buildGroups(data);
  state.ui.activeDate = null;
  state.ui.activeTopic = null;
  resetDirTree();
  applyListFilters();
}

describe('buildDirTree', () => {
  it('nests projects → sub-folders → documents from raw_path', () => {
    const roots = buildDirTree(data, titles);
    expect(roots.map((r) => r.name)).toEqual(['AI', 'Android', 'Empty project']);
    const builder = findDirNode(roots, 'android/arch/build/builder');
    expect(builder.kind).toBe('folder');
    expect(builder.docs.map((d) => d.name)).toEqual(['detekt.md', 'lint.md']);
    expect(findDirNode(roots, 'android/arch').children.map((c) => c.name)).toEqual(['build']);
  });

  it('keeps several documents in one folder', () => {
    const roots = buildDirTree(data, titles);
    expect(findDirNode(roots, 'ai/topic-x').docs).toHaveLength(2);
    expect(findDirNode(roots, 'android').docs).toHaveLength(1);
    expect(findDirNode(roots, 'android/arch').docs).toHaveLength(1);
  });

  it('puts a document with an empty raw_path or a bare file name directly under its project', () => {
    const roots = buildDirTree(data, titles);
    expect(findDirNode(roots, 'ai').docs.map((d) => d.id)).toEqual(['g']);
    expect(findDirNode(roots, 'android').docs.map((d) => d.name)).toEqual(['readme.md']);
  });

  it('lists projects that hold no documents', () => {
    const empty = findDirNode(buildDirTree(data, titles), 'empty');
    expect(empty.kind).toBe('project');
    expect(empty.children).toEqual([]);
    expect(empty.docs).toEqual([]);
  });

  it('names a node key and its path', () => {
    expect(nodeKeyOfEntry(data.a)).toBe('android/arch/build/builder');
    expect(pathKeys('a/b/c')).toEqual(['a', 'a/b', 'a/b/c']);
  });
});

describe('directory view selection', () => {
  beforeEach(() => {
    setView('directory');
    load();
  });

  it('starts with no day selected and every folder collapsed', () => {
    expect(state.ui.activeDate).toBeNull();
    expect(state.index.filteredGroups).toEqual([]);
    expect(dirTreeStore.getSnapshot().expanded).toEqual([]);
  });

  it('opening a document reveals its path, selects its project and lists its own folder', () => {
    selectDate(dirKeyOf('android/arch/build/builder'));
    expect(state.ui.activeDate).toBe(dirKeyOf('android/arch/build/builder'));
    expect(state.ui.activeTopic).toBe('android');
    expect(dirTreeStore.getSnapshot().expanded).toEqual(['android', 'android/arch', 'android/arch/build', 'android/arch/build/builder']);
    const [group] = state.index.filteredGroups;
    expect(group.entries.map((e) => e.id)).toEqual(['b', 'a']);
  });

  it('an empty folder still has a group so the page can say so', () => {
    selectDate(dirKeyOf('empty'));
    expect(state.index.filteredGroups).toHaveLength(1);
    expect(state.index.filteredGroups[0].entries).toEqual([]);
  });

  it('opening a document from the tree uses its folder as the group key', () => {
    expect(entryGroupKey(data.a)).toBe(dirKeyOf('android/arch/build/builder'));
    setView('timeline');
    expect(entryGroupKey(data.a)).toBe('20260401');
    expect(entryGroupKey(data.c)).toBe('undated');
  });
});

describe('entering from Home', () => {
  it('directory view: a project opens selected; entering the workspace selects the first project', () => {
    setView('directory');
    load();
    selectProject('android');
    expect(state.ui.activeDate).toBeNull();
    expect(state.ui.activeTopic).toBe('android');
    expect(dirTreeStore.getSnapshot().expanded).toEqual([]);
    selectDefaultProject();
    expect(state.ui.activeTopic).toBe('ai');
    expect(state.ui.activeDate).toBeNull();
    expect(dirTreeStore.getSnapshot().expanded).toEqual([]);
    expect(state.index.filteredGroups).toEqual([]);
  });

  it('directory view of a workspace with no projects has nothing to select', () => {
    setView('directory');
    load();
    state.index.topicTitles = {};
    selectDefaultProject();
    expect(state.ui.activeTopic).toBeNull();
  });

  it('timeline view: a project fills the picker, entering the workspace clears it', () => {
    setView('timeline');
    load();
    selectProject('ai');
    expect(state.ui.activeTopic).toBe('ai');
    expect(state.index.filteredGroups.every((g) => g.entries.every(({ entry }) => entry.folder === 'ai'))).toBe(true);
    selectDefaultProject();
    expect(state.ui.activeTopic).toBeNull();
    expect(state.index.filteredGroups).toBe(state.index.groupedByDate);
  });
});

describe('switching the view mode keeps the selected project', () => {
  it('timeline → directory selects the project picked in the picker', () => {
    setView('timeline');
    load();
    selectTopic('ai');
    setView('directory');
    applyViewModeChange();
    expect(state.ui.activeTopic).toBe('ai');
    expect(state.ui.activeDate).toBeNull();
  });

  it('directory → timeline carries the project to the picker and regroups by day', () => {
    setView('directory');
    load();
    selectProject('android');
    setView('timeline');
    applyViewModeChange();
    expect(state.ui.activeTopic).toBe('android');
    expect(state.index.filteredGroups.every((g) => !g.date.startsWith('dir:'))).toBe(true);
    for (const g of state.index.filteredGroups) {
      expect(g.entries.every(({ entry }) => entry.folder === 'android')).toBe(true);
    }
  });

  it('with no project selected the timeline lists everything and the directory takes the first project', () => {
    setView('directory');
    load();
    setView('timeline');
    applyViewModeChange();
    expect(state.index.filteredGroups).toBe(state.index.groupedByDate);
    setView('directory');
    applyViewModeChange();
    expect(state.ui.activeDate).toBeNull();
    expect(state.ui.activeTopic).toBe('ai');
    expect(state.index.filteredGroups).toEqual([]);
  });
});

describe('DirTree', () => {
  function render() {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    act(() => root.render(createElement(DirTree)));
    return { host, root };
  }

  beforeEach(() => {
    document.body.innerHTML = '';
    setView('directory');
    load();
  });

  const names = (host) => [...host.querySelectorAll('.knowledge-doc-tree-name')].map((n) => n.textContent);

  it('paints the selected project only: no project node, folders before files', () => {
    const { host } = render();
    act(() => selectDefaultProject());
    expect(names(host)).toEqual(['topic-x', 'Untitled note']);
    act(() => selectProject('android'));
    expect(names(host)).toEqual(['arch', 'readme.md']);
    expect(host.querySelector('[data-node="android"]')).toBeNull();
    act(() => selectProject('ai'));
    expect(names(host)).toEqual(['topic-x', 'Untitled note']);
    expect(host.querySelectorAll('.is-file')).toHaveLength(1);
  });

  it('says so when the selected project holds no documents', () => {
    const { host } = render();
    act(() => selectProject('empty'));
    expect(host.querySelector('.knowledge-doc-empty').textContent).toContain('no documents');
  });

  it('switching project resets what was opened', () => {
    const { host } = render();
    act(() => selectProject('android'));
    act(() => host.querySelector('[data-node="android/arch"] > button').click());
    expect(dirTreeStore.getSnapshot().expanded).toEqual(['android/arch']);
    act(() => selectProject('ai'));
    expect(dirTreeStore.getSnapshot().expanded).toEqual([]);
  });

  it('a folder only opens and closes: files and sub-folders appear, no list is selected', () => {
    const { host } = render();
    act(() => selectProject('android'));
    act(() => host.querySelector('[data-node="android/arch"] > button').click());
    expect(state.ui.activeDate).toBeNull();
    expect(host.querySelector('[data-node="android/arch/build"]')).not.toBeNull();
    expect(host.querySelector('.is-file[data-id="c"] .knowledge-doc-tree-stem').textContent).toBe('intro');
    expect(host.querySelector('.is-file[data-id="c"] .knowledge-doc-tree-ext').textContent).toBe('.md');
    act(() => host.querySelector('[data-node="android/arch"] > button').click());
    expect(host.querySelector('[data-node="android/arch/build"]')).toBeNull();
  });

  it('has no edit actions', () => {
    const { host } = render();
    act(() => selectProject('android'));
    act(() => host.querySelector('[data-node="android/arch"] > button').click());
    expect(host.querySelector('input, [data-action], .knowledge-doc-tree-rename')).toBeNull();
  });

  it('opens a document through the shared open-entry event', () => {
    const { host } = render();
    act(() => selectProject('android'));
    const seen = vi.fn();
    document.addEventListener('cta:open-entry', (e) => seen(e.detail));
    act(() => host.querySelector('.is-file[data-id="d"] button').click());
    expect(seen).toHaveBeenCalledWith({ id: 'd', layer: 'raw' });
  });

  it('keeps day selection working in the timeline view', () => {
    setView('timeline');
    selectDate('20260401');
    expect(state.ui.activeDate).toBe('20260401');
    act(() => notifyState());
  });
});

describe('directory tree style', () => {
  const css = readFileSync(join(process.cwd(), 'frontend/app.css'), 'utf8');

  it("uses main's Knowledge tree rules and none of the rewritten ones", () => {
    expect(css).toMatch(/\.knowledge-doc-tree-label \{[\s\S]*?padding: 7px 8px 7px 4px;/);
    expect(css).toMatch(/\.knowledge-doc-tree-children::before \{[\s\S]*?left: var\(--knowledge-tree-guide, 12px\);/);
    expect(css).not.toMatch(/\.dir-(row|chevron|name|count|doc|empty)\b/);
  });
});
