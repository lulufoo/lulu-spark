// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { buildCard, updateTitlesInDOM, sourceTypeBadgeHtml } from '../frontend/js/components/cards.js';
import { state } from '../frontend/js/state.js';

vi.mock('../frontend/js/api.js', () => ({
  setImportance: vi.fn(),
  setDone: vi.fn(),
  fetchFileContent: vi.fn(),
}));

vi.mock('../frontend/js/components/modals/move-project-dialog.js', () => ({
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
    expect(html).toContain('对话');
    expect(html).toMatch(/^<span/);
  });

  it('theme-line 使用 themeline class', () => {
    const html = sourceTypeBadgeHtml('theme-line');
    expect(html).toContain('badge-source-themeline');
    expect(html).toContain('视频');
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
    expect(first.textContent).toBe('对话');
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
    expect(card.querySelector('.badge-source-summary')?.textContent).toBe('总结');
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
    expect(first.textContent).toBe('对话');
  });
});
