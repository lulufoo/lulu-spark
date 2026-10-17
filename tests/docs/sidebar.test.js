// @vitest-environment jsdom
import { act } from 'react';
import { describe, it, expect, beforeEach, vi } from 'vitest';

vi.mock('../../frontend/src/shared/utils.ts', async (importOriginal) => ({
  ...(await importOriginal()),
  formatDate: () => ({ full: '2025-01-01', label: '01-01', year: '2025', weekday: 'Wed' }),
}));
vi.mock('../../frontend/src/notes/ui/cards.tsx', () => ({ renderDocList: vi.fn(), loadTitles: vi.fn() }));
vi.mock('../../frontend/src/notes/commands/cards.ts', () => ({ loadTitles: vi.fn() }));
vi.mock('../../frontend/src/notes/ui/sidebar-resize.ts', () => ({
  attachNotesSidebarResize: vi.fn(() => () => {}),
  detachNotesSidebarResize: vi.fn(),
  initSidebarResize: vi.fn(),
}));
vi.mock('../../frontend/src/router/index.ts', () => ({
  parseHash: vi.fn(() => ({ name: 'spark', params: {} })),
  navigateToDateList: vi.fn(),
}));

import { notifyState, state } from '../../frontend/src/host/state.ts';
import { parseHash, navigateToDateList } from '../../frontend/src/router/index.ts';
import { renderSidebar } from '../../frontend/src/notes/ui/sidebar.tsx';
import {
  applyListFilters,
  buildGroups,
  selectDate,
  selectTopic,
} from '../../frontend/src/notes/commands/sidebar.ts';
import { renderDocList } from '../../frontend/src/notes/ui/cards.tsx';

const makeGroup = (date, topics, tagKeys = []) => ({
  date,
  entries: topics.map((t, i) => ({
    id: `e${i}`,
    entry: {
      folder: t,
      created_at: date + '120000',
      tag_keys: tagKeys[i] ? [tagKeys[i]] : undefined,
    },
  })),
});

function seedDom() {
  document.body.innerHTML = `
    <aside id="sidebar"></aside>
    <div id="status"></div>
    <div id="date-heading"></div>
    <div id="doc-list"></div>
  `;
}

function sidebarEl() {
  return document.getElementById('sidebar');
}

function findSelect(selector) {
  return sidebarEl()?.querySelector(selector);
}

function optionTexts(picker) {
  return (picker?._listSelectOptions || []).map((o) => o.label);
}

function seedWorkspaces(viewMode) {
  state.workspaces = {
    current: 'default',
    list: [
      { id: 'default', name: 'Default', view_mode: viewMode, is_default: true },
      { id: 'research', name: 'Research', view_mode: 'timeline', is_default: false },
    ],
  };
}

beforeEach(() => {
  seedDom();
  state.index.data = {
    a: { folder: 'ai', created_at: '202501011200', tag_keys: ['k1'] },
    b: { folder: 'ai', created_at: '202501021200', tag_keys: ['k1', 'k2'] },
    c: { folder: 'common-tech', created_at: '202501011200', tag_keys: ['k2'] },
    d: { created_at: '202501031200' },
  };
  state.index.groupedByDate = [
    makeGroup('20250101', ['ai', 'common-tech'], ['k1', 'k2']),
    makeGroup('20250102', ['ai'], ['k1']),
    makeGroup('20250103', ['unknown'], []),
  ];
  state.index.filteredGroups = state.index.groupedByDate;
  state.index.tagsRegistry = { keys: { k1: { value: 'Alpha' }, k2: { value: 'Beta' } } };
  seedWorkspaces('timeline');
  state.ui.activeTopic = null;
  state.ui.activeDate = null;
  state.ui.sparkRoot = '';
});

describe('selectTopic', () => {
  it('key=null 时 filteredGroups === groupedByDate', () => {
    selectTopic(null);
    expect(state.index.filteredGroups).toBe(state.index.groupedByDate);
  });

  it('key="ai" 时只保留含 ai 条目的 groups', () => {
    selectTopic('ai');
    expect(state.ui.activeTopic).toBe('ai');
    for (const g of state.index.filteredGroups) {
      expect(g.entries.every(({ entry }) => (entry.folder || 'unknown') === 'ai')).toBe(true);
    }
  });

  it('key="unknown" 时过滤没有 folder 的条目', () => {
    selectTopic('unknown');
    expect(state.ui.activeTopic).toBe('unknown');
    expect(state.index.filteredGroups.length).toBeGreaterThan(0);
    for (const g of state.index.filteredGroups) {
      expect(g.entries.every(({ entry }) => (entry.folder || 'unknown') === 'unknown')).toBe(true);
    }
  });

  it('key 无匹配时 filteredGroups 为 []', () => {
    selectTopic('nonexistent-topic');
    expect(state.index.filteredGroups).toHaveLength(0);
  });
});

describe('timeline view', () => {
  it('narrows every day to the active project', () => {
    state.ui.activeTopic = 'common-tech';
    applyListFilters();
    expect(state.index.filteredGroups).toHaveLength(1);
    expect(state.index.filteredGroups[0].date).toBe('20250101');
  });

  it('lists every document when no project is selected', () => {
    applyListFilters();
    expect(state.index.filteredGroups).toBe(state.index.groupedByDate);
  });
});

describe('directory view', () => {
  it('lists nothing until a tree node is selected, whatever the project picker held', () => {
    seedWorkspaces('directory');
    state.ui.activeTopic = 'ai';
    applyListFilters();
    expect(state.index.filteredGroups).toEqual([]);
  });

  it('ignores day keys: the tree decides what is selected', () => {
    seedWorkspaces('directory');
    selectDate('20250101');
    expect(state.ui.activeDate).toBeNull();
  });
});

describe('renderSidebar workspace switch', () => {
  it('lists the workspaces by name and marks the current view mode', () => {
    applyListFilters();
    renderSidebar();
    expect(optionTexts(findSelect('.workspace-select'))).toEqual(['Default', 'Research']);
    const active = sidebarEl()?.querySelector('.view-mode-btn.active');
    expect(active?.dataset.viewMode).toBe('timeline');
  });

  it('counts undated documents and lists them under an Undated tab', () => {
    state.index.data = {
      a: { folder: 'ai', title: 'A' },
      b: { folder: 'ai', title: 'B' },
      c: { folder: 'ops', title: 'C', created_at: '202501011200' },
    };
    state.index.groupedByDate = buildGroups(state.index.data);
    applyListFilters();
    renderSidebar();
    expect(optionTexts(findSelect('.topic-select'))[0]).toBe('All (3)');
    const tabs = [...sidebarEl().querySelectorAll('.date-tab')].map((t) => t.dataset.date);
    expect(tabs).toEqual(['20250101', 'undated']);
    expect(sidebarEl().querySelector('.date-tab[data-date="undated"] .count').textContent).toBe('2');
  });

  it('shows the project picker in the timeline view only; the directory view paints the tree', () => {
    applyListFilters();
    renderSidebar();
    expect(sidebarEl()?.querySelectorAll('.topic-filter')).toHaveLength(1);
    expect(optionTexts(findSelect('.topic-select'))[0]).toBe('All (4)');
    expect(sidebarEl()?.querySelector('#sidebar-dir-tree')).toBeNull();

    seedWorkspaces('directory');
    renderSidebar();
    act(() => notifyState());
    expect(sidebarEl()?.querySelectorAll('.topic-filter')).toHaveLength(0);
    expect(sidebarEl()?.querySelector('.date-tab')).toBeNull();
    expect(sidebarEl()?.querySelector('#sidebar-dir-tree')).not.toBeNull();
    expect(sidebarEl()?.querySelector('.view-mode-btn.active')?.dataset.viewMode).toBe('directory');
  });

  it('has no tag filter any more', () => {
    applyListFilters();
    renderSidebar();
    expect(sidebarEl()?.querySelectorAll('.tag-filter, .tag-select')).toHaveLength(0);
  });

  it('remount keeps one workspace picker and one project picker', () => {
    applyListFilters();
    renderSidebar();
    renderSidebar();
    expect(sidebarEl()?.querySelectorAll('.workspace-filter')).toHaveLength(1);
    expect(sidebarEl()?.querySelectorAll('.topic-filter')).toHaveLength(1);
    expect(sidebarEl()?.querySelectorAll('.list-select-picker')).toHaveLength(2);
  });

  it('clears leftover filters when the channel effect re-paints', () => {
    applyListFilters();
    renderSidebar();
    const stray = document.createElement('div');
    stray.className = 'topic-filter';
    stray.dataset.stray = '1';
    sidebarEl()?.append(stray);
    act(() => {
      state.workspaces = { ...state.workspaces, list: [...state.workspaces.list] };
      notifyState();
    });
    expect(sidebarEl()?.querySelector('[data-stray]')).toBeNull();
    expect(sidebarEl()?.querySelectorAll('.topic-filter')).toHaveLength(1);
    expect(sidebarEl()?.querySelectorAll('.list-select-picker')).toHaveLength(2);
  });
});

describe('sidebar channel nav', () => {
  function channelTabChannels() {
    return [...(sidebarEl()?.querySelectorAll('.sidebar-channel-tab') || [])].map(
      (node) => node.dataset.channel,
    );
  }

  it('does not render read-later channel tab', () => {
    applyListFilters();
    renderSidebar();
    expect(channelTabChannels()).not.toContain('read-later');
  });

  it('does not render archive channel tab', () => {
    applyListFilters();
    renderSidebar();
    expect(channelTabChannels()).not.toContain('archive');
    expect(channelTabChannels()).toHaveLength(0);
  });

  it('does not export selectReadLaterChannel', async () => {
    const mod = await import('../../frontend/src/notes/ui/sidebar.tsx');
    expect(mod.selectReadLaterChannel).toBeUndefined();
  });
});

describe('buildGroups', () => {
  it('puts documents without created_at into an Undated group after every date', () => {
    const grouped = buildGroups({
      ok: { common_path: 'ai/a', created_at: '202501011200' },
      older: { common_path: 'ai/c', created_at: '202412311200' },
      zeta: { common_path: 'ai/z', title: 'Zeta' },
      alpha: { common_path: 'ai/b', title: 'Alpha', created_at: null },
    });
    expect(grouped.map((g) => g.date)).toEqual(['20250101', '20241231', 'undated']);
    expect(grouped[2].entries.map((e) => e.id)).toEqual(['alpha', 'zeta']);
  });

  it('keeps a workspace whose documents are all undated listable', () => {
    const grouped = buildGroups({ a: { common_path: 'x/a' }, b: { common_path: 'x/b' } });
    expect(grouped).toHaveLength(1);
    expect(grouped[0].date).toBe('undated');
    expect(grouped[0].entries).toHaveLength(2);
  });
});

describe('selectDate while note/create active → list via location', () => {
  beforeEach(() => {
    vi.mocked(parseHash).mockReset();
    vi.mocked(navigateToDateList).mockReset();
    vi.mocked(renderDocList).mockClear();
    state.viewer = { createSession: null };
    state.index = {
      filteredGroups: [makeGroup('20260719', ['inbox']), makeGroup('20260718', ['inbox'])],
      groupedByDate: [],
    };
    state.ui = { activeDate: null, activeTopic: null };
  });

  it('with note in hash: navigates to date list (does not only refresh doc-list)', () => {
    vi.mocked(parseHash).mockReturnValue({
      name: 'spark',
      params: { date: '20260719', note: 'inbox/notes/x.md' },
    });
    selectDate('20260718');
    expect(navigateToDateList).toHaveBeenCalledWith('20260718');
    expect(renderDocList).not.toHaveBeenCalled();
  });

  it('create in progress: clears createSession and navigates to date list', () => {
    state.viewer.createSession = { tempId: 'tmp', status: 'creating' };
    vi.mocked(parseHash).mockReturnValue({
      name: 'spark',
      params: { date: '20260719' },
    });
    selectDate('20260719');
    expect(state.viewer.createSession).toBeNull();
    expect(navigateToDateList).toHaveBeenCalledWith('20260719');
    expect(renderDocList).not.toHaveBeenCalled();
  });

  it('list mode (no note, not creating): updates list without navigateToDateList', () => {
    vi.mocked(parseHash).mockReturnValue({
      name: 'spark',
      params: { date: '20260719' },
    });
    selectDate('20260718');
    expect(navigateToDateList).not.toHaveBeenCalled();
    expect(renderDocList).not.toHaveBeenCalled();
    expect(state.ui.activeDate).toBe('20260718');
  });
});
