import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock DOM dependencies before importing sidebar
vi.mock('../frontend/js/utils.js', () => ({ formatDate: () => ({ full: '2025-01-01' }) }));
vi.mock('../frontend/js/components/cards.js', () => ({ renderDocList: vi.fn(), loadTitles: vi.fn() }));

// Provide minimal document stub (supports _ensureSidebarZones)
const makeEl = (tag = 'div') => {
  const el = {
    tag,
    id: '',
    className: '',
    value: '',
    textContent: '',
    title: '',
    disabled: false,
    tabIndex: 0,
    children: [],
    style: {},
    dataset: {},
    innerHTML: '',
  };
  const matchSel = (node, sel) => {
    if (sel.startsWith('#') && node.id === sel.slice(1)) return node;
    if (sel.startsWith('.') && node.className === sel.slice(1)) return node;
    return null;
  };
  const query = (sel) => {
    const direct = matchSel(el, sel);
    if (direct) return direct;
    for (const child of el.children) {
      const found = child.querySelector?.(sel);
      if (found) return found;
    }
    return null;
  };
  el.appendChild = (child) => {
    if (child.id) el[`#${child.id}`] = child;
    el.children.push(child);
    return child;
  };
  el.addEventListener = vi.fn();
  el.setAttribute = vi.fn();
  el.querySelector = query;
  el.querySelectorAll = () => [];
  return el;
};

const sidebarEl = makeEl('aside');
sidebarEl.id = 'sidebar';

globalThis.document = {
  getElementById: (id) => (id === 'sidebar' ? sidebarEl : makeEl()),
  createElement: (tag) => makeEl(tag),
  querySelectorAll: () => [],
};

globalThis.sessionStorage = { getItem: () => null, setItem: () => {} };

import { state } from '../frontend/js/state.js';
import { buildGroups, selectTopic } from '../frontend/js/components/sidebar.js';

const makeGroup = (date, topics) => ({
  date,
  entries: topics.map((t, i) => ({ id: `e${i}`, entry: { common_path: `${t}/foo`, created_at: date + '120000' } })),
});

beforeEach(() => {
  sidebarEl.children = [];
  sidebarEl.innerHTML = '';
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
  state.ui.workbenchKnowledgeRoot = '';
  state.ui.knowledgeCorpusRoot = '';
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
