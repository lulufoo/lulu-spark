// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
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
const { applyListFilters, buildGroups, selectDate, selectDirNode, selectTopic } = await import(
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
    expect(builder.docs.map((d) => d.name)).toEqual(['detekt', 'lint']);
    expect(findDirNode(roots, 'android/arch').children.map((c) => c.name)).toEqual(['build']);
  });

  it('keeps several documents in one folder and counts the whole subtree', () => {
    const roots = buildDirTree(data, titles);
    expect(findDirNode(roots, 'ai/topic-x').docs).toHaveLength(2);
    expect(findDirNode(roots, 'android').count).toBe(4);
    expect(findDirNode(roots, 'android/arch').count).toBe(3);
  });

  it('puts a document with an empty raw_path or a bare file name directly under its project', () => {
    const roots = buildDirTree(data, titles);
    expect(findDirNode(roots, 'ai').docs.map((d) => d.id)).toEqual(['g']);
    expect(findDirNode(roots, 'android').docs.map((d) => d.name)).toEqual(['readme']);
  });

  it('lists projects that hold no documents', () => {
    const empty = findDirNode(buildDirTree(data, titles), 'empty');
    expect(empty.kind).toBe('project');
    expect(empty.count).toBe(0);
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

  it('starts with nothing selected and every project collapsed', () => {
    expect(state.ui.activeDate).toBeNull();
    expect(state.index.filteredGroups).toEqual([]);
    expect(dirTreeStore.getSnapshot().expanded).toEqual([]);
  });

  it('selecting a folder opens its path, selects its project and lists its own documents', () => {
    selectDirNode('android/arch/build/builder');
    expect(state.ui.activeDate).toBe(dirKeyOf('android/arch/build/builder'));
    expect(state.ui.activeTopic).toBe('android');
    expect(dirTreeStore.getSnapshot().expanded).toEqual(['android', 'android/arch', 'android/arch/build', 'android/arch/build/builder']);
    const [group] = state.index.filteredGroups;
    expect(group.entries.map((e) => e.id)).toEqual(['b', 'a']);
  });

  it('an empty folder still has a group so the page can say so', () => {
    selectDirNode('empty');
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

describe('switching the view mode keeps the selected project', () => {
  it('timeline → directory selects the project picked in the picker', () => {
    setView('timeline');
    load();
    selectTopic('ai');
    setView('directory');
    applyViewModeChange();
    expect(state.ui.activeDate).toBe(dirKeyOf('ai'));
    expect(state.index.filteredGroups[0].entries.map((e) => e.id)).toEqual(['g']);
  });

  it('directory → timeline carries the project to the picker and regroups by day', () => {
    setView('directory');
    load();
    selectDirNode('android/arch');
    setView('timeline');
    applyViewModeChange();
    expect(state.ui.activeTopic).toBe('android');
    expect(state.index.filteredGroups.every((g) => !g.date.startsWith('dir:'))).toBe(true);
    for (const g of state.index.filteredGroups) {
      expect(g.entries.every(({ entry }) => entry.folder === 'android')).toBe(true);
    }
  });

  it('with no project selected the directory stays unselected and the timeline lists everything', () => {
    setView('directory');
    load();
    setView('timeline');
    applyViewModeChange();
    expect(state.index.filteredGroups).toBe(state.index.groupedByDate);
    setView('directory');
    applyViewModeChange();
    expect(state.ui.activeDate).toBeNull();
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

  it('paints only the projects while everything is collapsed', () => {
    const { host } = render();
    expect([...host.querySelectorAll('.dir-project > .dir-row .dir-name')].map((n) => n.textContent)).toEqual([
      'AI',
      'Android',
      'Empty project',
    ]);
    expect(host.querySelectorAll('.dir-folder, .dir-doc')).toHaveLength(0);
  });

  it('opens the path, highlights the node and shows its documents when a folder is selected', () => {
    const { host } = render();
    act(() => selectDirNode('android/arch'));
    const active = host.querySelector('.dir-row.active');
    expect(active.closest('[data-node]').dataset.node).toBe('android/arch');
    expect(host.querySelector('[data-node="android/arch/build"]')).not.toBeNull();
    expect(host.querySelector('.dir-doc[data-id="c"]')).not.toBeNull();
  });

  it('collapses the selected node on a second click and has no edit actions', () => {
    const { host } = render();
    act(() => selectDirNode('ai'));
    const row = host.querySelector('[data-node="ai"] > .dir-row');
    act(() => row.click());
    expect(dirTreeStore.getSnapshot().expanded).not.toContain('ai');
    expect(host.querySelector('[data-action], .dir-rename, .dir-delete')).toBeNull();
  });

  it('opens a document through the shared open-entry event', () => {
    const { host } = render();
    act(() => selectDirNode('ai'));
    const seen = vi.fn();
    document.addEventListener('cta:open-entry', (e) => seen(e.detail));
    act(() => host.querySelector('.dir-doc[data-id="g"]').click());
    expect(seen).toHaveBeenCalledWith({ id: 'g', layer: 'raw' });
  });

  it('keeps day selection working in the timeline view', () => {
    setView('timeline');
    selectDate('20260401');
    expect(state.ui.activeDate).toBe('20260401');
    act(() => notifyState());
  });
});
