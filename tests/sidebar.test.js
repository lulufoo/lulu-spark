import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock DOM dependencies before importing sidebar
vi.mock('../js/utils.js', () => ({ formatDate: () => ({ full: '2025-01-01' }) }));
vi.mock('../js/components/cards.js', () => ({ renderDocList: vi.fn(), loadTitles: vi.fn() }));

// Provide minimal document stub
const makeEl = (tag = 'div') => {
  const el = { tag, className: '', value: '', textContent: '', title: '', disabled: false, children: [], style: {}, dataset: {}, innerHTML: '' };
  el.appendChild = (child) => el.children.push(child);
  el.addEventListener = vi.fn();
  el.querySelectorAll = () => [];
  return el;
};

globalThis.document = {
  getElementById: () => makeEl(),
  createElement: (tag) => makeEl(tag),
  querySelectorAll: () => [],
};

globalThis.sessionStorage = { getItem: () => null, setItem: () => {} };

import { state } from '../js/state.js';
import { selectTopic } from '../js/components/sidebar.js';

const makeGroup = (date, topics) => ({
  date,
  entries: topics.map((t, i) => ({ id: `e${i}`, entry: { common_path: `${t}/foo`, created_at: date + '120000' } })),
});

beforeEach(() => {
  state.index.data = {
    a: { common_path: 'ai/x', created_at: '202501011200' },
    b: { common_path: 'ai/y', created_at: '202501021200' },
    c: { common_path: 'common-tech/z', created_at: '202501011200' },
    d: { common_path: null, created_at: '202501031200' },
  };
  state.index.groupedByDate = [
    makeGroup('20250101', ['ai', 'common-tech']),
    makeGroup('20250102', ['ai']),
    makeGroup('20250103', ['unknown']),
  ];
  state.index.filteredGroups = state.index.groupedByDate;
  state.ui.activeTopic = null;
  state.ui.activeDate = null;
  state.ui.archiveRoot = '';
  state.ui.kbRoot = '';
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
