// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { buildCard, updateTitlesInDOM, loadTitles, sourceTypeBadgeHtml } from '../../frontend/src/notes/cards.tsx';
import { state } from '../../frontend/src/host/state.ts';
import * as api from '../../frontend/src/host/api.ts';

vi.mock('../../frontend/src/host/api.ts', () => ({
  setImportance: vi.fn(),
  setDone: vi.fn(),
  fetchFileContent: vi.fn(),
}));

vi.mock('../../frontend/src/notes/move-project-dialog.tsx', () => ({
  openMoveProjectDialog: vi.fn(),
}));

beforeEach(() => {
  document.body.innerHTML = '';
  const list = document.createElement('div');
  list.id = 'doc-list';
  document.body.appendChild(list);

  state.index.topicDescriptions = {};
  state.index.diffStatus = new Map();
  state.index.titleCache = new Map();
  state.index.groupedByDate = [];
  state.ui.activeDate = '20260518';
});

describe('sourceTypeBadgeHtml', () => {
  it('dialogue 返回对话 badge HTML', () => {
    const html = sourceTypeBadgeHtml('dialogue');
    expect(html).toContain('badge-source-dialogue');
    expect(html).toContain('Dialogue');
    expect(html).toMatch(/^<span/);
  });

  it('theme-line 使用 themeline class', () => {
    const html = sourceTypeBadgeHtml('theme-line');
    expect(html).toContain('badge-source-themeline');
    expect(html).toContain('Video');
  });

  it('未知类型返回空字符串', () => {
    expect(sourceTypeBadgeHtml('unknown')).toBe('');
    expect(sourceTypeBadgeHtml(undefined)).toBe('');
  });
});

describe('buildCard source_type badge', () => {
  it('dialogue 显示蓝色「对话」badge 且在 badges 行最前', () => {
    const entry = {
      common_path: 'proj/topic/202605181200-note.md',
      created_at: '202605181200',
      layers: ['raw'],
      source_type: 'dialogue',
    };
    const card = buildCard('id1', entry, 'Title');
    const badges = card.querySelector('.badges');
    const first = badges.firstElementChild;
    expect(first.classList.contains('badge-source')).toBe(true);
    expect(first.classList.contains('badge-source-dialogue')).toBe(true);
    expect(first.textContent).toBe('Dialogue');
    expect(first.tagName).toBe('SPAN');
  });

  it('summary 显示黄色「总结」badge', () => {
    const entry = {
      common_path: 'proj/topic/202605181200-note.md',
      created_at: '202605181200',
      layers: ['raw'],
      source_type: 'summary',
    };
    const card = buildCard('id2', entry, 'Title');
    expect(card.querySelector('.badge-source-summary')?.textContent).toBe('Summary');
  });

  it('无 source_type 不显示来源 badge', () => {
    const entry = {
      common_path: 'proj/topic/202605181200-note.md',
      created_at: '202605181200',
      layers: ['raw'],
    };
    const card = buildCard('id4', entry, 'Title');
    expect(card.querySelector('.badge-source')).toBeNull();
  });
});

describe('loadTitles incremental cache', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('已有 date 缓存时仍为新条目拉取 H1 标题', async () => {
    const date = '20260522';
    state.ui.activeDate = date;
    state.index.titleCache.set(date, new Map([['old-id', 'Cached Title']]));

    const newEntry = {
      common_path: 'ai-software-dev/diagnostic-gate-model-design/202605222313-diagnostic-gate-model-design.md',
      created_at: '202605222313',
      layers: ['raw'],
    };
    state.index.groupedByDate = [{ date, entries: [{ id: 'new-id', entry: newEntry }] }];

    api.fetchFileContent.mockResolvedValueOnce('# Real H1 From Raw\n\nbody');
    await loadTitles([{ id: 'new-id', entry: newEntry }], date);

    expect(api.fetchFileContent).toHaveBeenCalledTimes(1);
    expect(state.index.titleCache.get(date).get('new-id')).toBe('Real H1 From Raw');
    expect(state.index.titleCache.get(date).get('old-id')).toBe('Cached Title');
  });

  it('缓存已覆盖全部条目时不重复请求', async () => {
    const date = '20260522';
    state.index.titleCache.set(date, new Map([['id1', 'Done']]));
    await loadTitles([{ id: 'id1', entry: { common_path: 'p/t/202605221200-x.md', layers: ['raw'] } }], date);
    expect(api.fetchFileContent).not.toHaveBeenCalled();
  });
});

describe('updateTitlesInDOM source_type badge', () => {
  it('标题更新后仍保留来源 badge', () => {
    const entry = {
      common_path: 'proj/topic/202605181200-note.md',
      created_at: '202605181200',
      layers: ['raw'],
      source_type: 'dialogue',
    };
    const card = buildCard('id6', entry, null);
    document.getElementById('doc-list').appendChild(card);

    state.index.titleCache.set('20260518', new Map([['id6', 'Loaded Title']]));
    state.index.groupedByDate = [{ date: '20260518', entries: [{ id: 'id6', entry }] }];

    updateTitlesInDOM('20260518');

    const first = card.querySelector('.badges').firstElementChild;
    expect(first.classList.contains('badge-source-dialogue')).toBe(true);
    expect(first.textContent).toBe('Dialogue');
  });
});
