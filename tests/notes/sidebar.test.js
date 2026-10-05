// @vitest-environment jsdom
import { act } from 'react';
import { describe, it, expect, beforeEach, vi } from 'vitest';

vi.mock('../../frontend/src/shared/utils.ts', () => ({
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
  clearTagFilter,
  selectDate,
  selectTag,
  selectTopic,
} from '../../frontend/src/notes/commands/sidebar.ts';
import { renderDocList } from '../../frontend/src/notes/ui/cards.tsx';

const makeGroup = (date, topics, tagKeys = []) => ({
  date,
  entries: topics.map((t, i) => ({
    id: `e${i}`,
    entry: {
      common_path: `${t}/foo`,
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

function findTagSelect() {
  return sidebarEl()?.querySelector('.tag-select');
}

function tagOptionTexts(picker) {
  return (picker?._listSelectOptions || []).map((o) => o.label);
}

beforeEach(() => {
  seedDom();
  state.index.data = {
    a: { common_path: 'ai/x', created_at: '202501011200', tag_keys: ['k1'] },
    b: { common_path: 'ai/y', created_at: '202501021200', tag_keys: ['k1', 'k2'] },
    c: { common_path: 'common-tech/z', created_at: '202501011200', tag_keys: ['k2'] },
    d: { common_path: null, created_at: '202501031200' },
  };
  state.index.groupedByDate = [
    makeGroup('20250101', ['ai', 'common-tech'], ['k1', 'k2']),
    makeGroup('20250102', ['ai'], ['k1']),
    makeGroup('20250103', ['unknown'], []),
  ];
  state.index.filteredGroups = state.index.groupedByDate;
  state.index.tagsRegistry = { keys: { k1: { value: 'Alpha' }, k2: { value: 'Beta' } } };
  state.ui.activeTopic = null;
  state.ui.activeTagKey = null;
  state.ui.activeDate = null;
  state.ui.sparkRoot = '';
  state.ui.knowledgeRoot = '';
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
      expect(g.entries.every(({ entry }) => entry.common_path?.split('/')[0] === 'ai')).toBe(true);
    }
  });

  it('key="unknown" 时过滤 common_path 为 null 的条目', () => {
    selectTopic('unknown');
    expect(state.ui.activeTopic).toBe('unknown');
    expect(state.index.filteredGroups.length).toBeGreaterThan(0);
    for (const g of state.index.filteredGroups) {
      expect(g.entries.every(({ entry }) => (entry.common_path?.split('/')[0] || 'unknown') === 'unknown')).toBe(true);
    }
  });

  it('key 无匹配时 filteredGroups 为 []', () => {
    selectTopic('nonexistent-topic');
    expect(state.index.filteredGroups).toHaveLength(0);
  });
});

describe('selectTag', () => {
  it('selectTag(k1) 后 filteredGroups 仅含 tag_keys 含 k1 的条目', () => {
    applyListFilters();
    selectTag('k1');
    expect(state.ui.activeTagKey).toBe('k1');
    for (const g of state.index.filteredGroups) {
      expect(g.entries.every(({ entry }) => entry.tag_keys?.includes('k1'))).toBe(true);
    }
  });

  it('clearTagFilter 清除 activeTagKey', () => {
    selectTag('k1');
    clearTagFilter();
    expect(state.ui.activeTagKey).toBeNull();
  });

  it('连续 selectTag(same) toggle 清除', () => {
    selectTag('k1');
    selectTag('k1');
    expect(state.ui.activeTagKey).toBeNull();
  });

  it('topic + tag 组合为交集', () => {
    selectTopic('ai');
    selectTag('k2');
    expect(state.ui.activeTopic).toBe('ai');
    expect(state.ui.activeTagKey).toBe('k2');
    for (const g of state.index.filteredGroups) {
      expect(
        g.entries.every(
          ({ entry }) => entry.common_path?.split('/')[0] === 'ai' && entry.tag_keys?.includes('k2'),
        ),
      ).toBe(true);
    }
  });

  it('无匹配 tag 时 filteredGroups 为空', () => {
    selectTag('nonexistent-tag');
    expect(state.index.filteredGroups).toHaveLength(0);
  });
});

describe('renderSidebar tag filter', () => {
  it('渲染 tag-select 与全部 (N) 选项', () => {
    applyListFilters();
    renderSidebar();
    const sel = findTagSelect();
    expect(sel).toBeTruthy();
    expect(tagOptionTexts(sel)[0]).toBe('All (4)');
  });

  it('按 label 字母序列出标签及 count', () => {
    applyListFilters();
    renderSidebar();
    const texts = tagOptionTexts(findTagSelect());
    expect(texts).toContain('Alpha (2)');
    expect(texts).toContain('Beta (2)');
    const alphaIdx = texts.indexOf('Alpha (2)');
    const betaIdx = texts.indexOf('Beta (2)');
    expect(alphaIdx).toBeLessThan(betaIdx);
  });

  it('activeTopic 下 N 与 count 仅统计该 topic', () => {
    state.ui.activeTopic = 'ai';
    applyListFilters();
    renderSidebar();
    const texts = tagOptionTexts(findTagSelect());
    expect(texts[0]).toBe('All (2)');
    expect(texts).toContain('Alpha (2)');
    expect(texts).not.toContain('Beta (2)');
  });

  it('scope 内无 tag 时仅全部 (N)', () => {
    state.index.data = {
      a: { common_path: 'ai/x', created_at: '202501011200' },
    };
    state.index.groupedByDate = [makeGroup('20250101', ['ai'], [])];
    state.index.filteredGroups = state.index.groupedByDate;
    applyListFilters();
    renderSidebar();
    const texts = tagOptionTexts(findTagSelect()).filter((t) => !t.includes('─'));
    expect(texts).toEqual(['All (1)']);
  });

  it('registry 缺失时回退 key', () => {
    state.index.tagsRegistry = { keys: {} };
    applyListFilters();
    renderSidebar();
    const texts = tagOptionTexts(findTagSelect());
    expect(texts.some((t) => t.startsWith('k1 ('))).toBe(true);
  });

  it('activeTagKey 时 tag-count 可见', () => {
    state.ui.activeTagKey = 'k1';
    applyListFilters();
    renderSidebar();
    const countEl = sidebarEl()?.querySelector('.tag-count');
    expect(countEl).toBeTruthy();
    expect(countEl.textContent).toBe('2 / 4 items');
    expect(countEl.style.display).not.toBe('none');
  });

  it('无 activeTagKey 时 tag-count 隐藏', () => {
    applyListFilters();
    renderSidebar();
    const countEl = sidebarEl()?.querySelector('.tag-count');
    expect(countEl.style.display).toBe('none');
  });

  it('remount keeps one topic picker and one tag picker', () => {
    applyListFilters();
    renderSidebar();
    renderSidebar();
    expect(sidebarEl()?.querySelectorAll('.topic-filter')).toHaveLength(1);
    expect(sidebarEl()?.querySelectorAll('.tag-filter')).toHaveLength(1);
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
      state.index.tagsRegistry = { keys: { k1: { value: 'Alpha' }, k2: { value: 'Beta' } } };
      notifyState();
    });
    expect(sidebarEl()?.querySelector('[data-stray]')).toBeNull();
    expect(sidebarEl()?.querySelectorAll('.topic-filter')).toHaveLength(1);
    expect(sidebarEl()?.querySelectorAll('.tag-filter')).toHaveLength(1);
    expect(sidebarEl()?.querySelectorAll('.list-select-picker')).toHaveLength(2);
  });

  it('orphan activeTagKey 追加 (0) option', () => {
    state.index.data = {
      a: { common_path: 'ai/x', created_at: '202501011200', tag_keys: ['k1'] },
    };
    state.index.groupedByDate = [makeGroup('20250101', ['ai'], ['k1'])];
    state.index.filteredGroups = state.index.groupedByDate;
    state.ui.activeTopic = 'ai';
    state.ui.activeTagKey = 'ghost';
    applyListFilters();
    renderSidebar();
    const sel = findTagSelect();
    expect(sel).toBeTruthy();
    expect(sel._listSelectValue).toBe('ghost');
    expect(tagOptionTexts(sel)).toContain('ghost (0)');
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
  it('忽略缺少 created_at 的异常条目', () => {
    const grouped = buildGroups({
      ok: { common_path: 'ai/a', created_at: '202501011200' },
      broken: { common_path: 'ai/b' },
    });
    expect(grouped).toHaveLength(1);
    expect(grouped[0].date).toBe('20250101');
    expect(grouped[0].entries).toHaveLength(1);
    expect(grouped[0].entries[0].id).toBe('ok');
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
    state.ui = { activeDate: null, activeTopic: null, activeTagKey: null };
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
