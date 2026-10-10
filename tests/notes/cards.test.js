// @vitest-environment jsdom
import { createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { buildCard, DocCard, updateTitlesInDOM, sourceTypeBadgeHtml, getEntryDiffState } from '../../frontend/src/notes/ui/cards.tsx';
import { digestCache } from '../../frontend/src/notes/ui/digest-tooltip.tsx';
import { loadTitles } from '../../frontend/src/notes/commands/cards.ts';
import { openNoteInChat } from '../../frontend/src/notes/commands/open-in-chat.ts';
import { state } from '../../frontend/src/host/state.ts';
import * as api from '../../frontend/src/host/api.ts';

vi.mock('../../frontend/src/host/api.ts', () => ({
  setImportance: vi.fn(),
  setDone: vi.fn(),
  fetchFileContent: vi.fn(),
}));

vi.mock('../../frontend/src/notes/ui/move-project-dialog.tsx', () => ({
  openMoveProjectDialog: vi.fn(),
  closeMoveProjectDialog: vi.fn(),
  MoveProjectDialog: () => null,
}));

vi.mock('../../frontend/src/notes/commands/open-in-chat.ts', () => ({
  openNoteInChat: vi.fn().mockResolvedValue(undefined),
}));

beforeEach(() => {
  document.body.innerHTML = '';
  const list = document.createElement('div');
  list.id = 'doc-list';
  document.body.appendChild(list);

  state.index.topicDescriptions = {};
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

describe('getEntryDiffState 只保留 unreachable', () => {
  const basePath = 'proj/topic/202605181200-note.md';

  function makeEntry(overrides = {}) {
    return {
      common_path: basePath,
      created_at: '202605181200',
      layers: ['raw'],
      ...overrides,
    };
  }

  it('_unreachable_raw 时 .doc-meta 渲染 .diff-dot.unreachable 与 ● unreachable 文案', () => {
    const entry = makeEntry({ _unreachable_raw: true });
    const card = buildCard('dd2', entry, 'Title');
    const dot = card.querySelector('.doc-meta .diff-dot');
    expect(getEntryDiffState(entry)).toBe('unreachable');
    expect(dot).not.toBeNull();
    expect(dot.classList.contains('unreachable')).toBe(true);
    expect(dot.textContent).toBe('● unreachable');
  });

  it('无 unreachable 标记时 .doc-meta 内无任何 .diff-dot 节点', () => {
    const card = buildCard('dd5', makeEntry(), 'Title');
    expect(card.querySelector('.doc-meta')).not.toBeNull();
    expect(card.querySelector('.doc-meta .diff-dot')).toBeNull();
    expect(card.querySelectorAll('.doc-meta .diff-dot').length).toBe(0);
  });

  it('无 layers / 空条目数据时 getEntryDiffState 返回 null 不抛错', () => {
    expect(getEntryDiffState({})).toBeNull();
    expect(getEntryDiffState({ common_path: basePath })).toBeNull();
  });
});

describe('digest tooltip 挂在最底徽章行（buildCard / DocCard）', () => {
  let reactRoot;

  function hover(el) {
    el.dispatchEvent(new MouseEvent('mouseover', { bubbles: true }));
    el.dispatchEvent(new MouseEvent('mouseenter'));
  }

  function leave(el) {
    el.dispatchEvent(new MouseEvent('mouseout', { bubbles: true }));
    el.dispatchEvent(new MouseEvent('mouseleave'));
  }

  function tooltipEl() {
    return document.querySelector('.digest-tooltip');
  }

  function makeEntry(commonPath, overrides = {}) {
    return {
      common_path: commonPath,
      created_at: '202605181200',
      layers: ['raw', 'digest'],
      source_type: 'note',
      ...overrides,
    };
  }

  function renderDocCard(id, entry, title) {
    const host = document.createElement('div');
    document.getElementById('doc-list').appendChild(host);
    reactRoot = createRoot(host);
    flushSync(() => {
      reactRoot.render(createElement(DocCard, { id, entry, title }));
    });
    return host.querySelector('.doc-card');
  }

  function hideBadgesQuery() {
    const orig = Element.prototype.querySelector;
    return vi.spyOn(Element.prototype, 'querySelector').mockImplementation(function (sel) {
      if (sel === '.badges') return null;
      return orig.call(this, sel);
    });
  }

  beforeEach(() => {
    vi.useFakeTimers();
    api.fetchFileContent.mockReset();
    digestCache.clear();
  });

  afterEach(() => {
    if (reactRoot) {
      flushSync(() => reactRoot.unmount());
      reactRoot = undefined;
    }
    vi.useRealTimers();
    document.querySelector('.digest-tooltip')?.remove();
  });

  it('buildCard：悬停 .badges 300ms 后显示纯文本预览；空 digest 无 DOM 反应', async () => {
    const cp = 'proj/topic/202605181200-tip.md';
    api.fetchFileContent.mockResolvedValueOnce('# **Plain** digest preview');
    const card = buildCard('tip1', makeEntry(cp), 'Title');
    document.getElementById('doc-list').appendChild(card);
    const badges = card.querySelector('.badges');

    hover(badges);
    await vi.advanceTimersByTimeAsync(299);
    expect(api.fetchFileContent).not.toHaveBeenCalled();
    expect(tooltipEl()).toBeNull();

    await vi.advanceTimersByTimeAsync(1);
    expect(api.fetchFileContent).toHaveBeenCalledWith('digest', cp);
    const tip = tooltipEl();
    expect(tip).not.toBeNull();
    expect(tip.textContent).toBe('Plain digest preview');

    leave(badges);
    expect(tooltipEl()).toBeNull();

    const cp2 = 'proj/topic/202605181201-tip2.md';
    api.fetchFileContent.mockResolvedValueOnce('');
    const card2 = buildCard('tip2', makeEntry(cp2, { created_at: '202605181201', layers: ['raw'] }), 'Title2');
    document.getElementById('doc-list').appendChild(card2);

    hover(card2.querySelector('.badges'));
    await vi.advanceTimersByTimeAsync(300);
    expect(api.fetchFileContent).toHaveBeenLastCalledWith('digest', cp2);
    expect(tooltipEl()).toBeNull();
  });

  it.each(['.doc-card', '.doc-topic', '.doc-title-btn', '.doc-meta'])(
    'buildCard：悬停 %s 不显示浮层、不发 digest 请求',
    async (sel) => {
      const cp = `proj/topic/202605181210-${sel.replace(/[^a-z]+/g, '')}.md`;
      api.fetchFileContent.mockResolvedValue('# digest');
      const card = buildCard(`b-${sel}`, makeEntry(cp), 'Title');
      document.getElementById('doc-list').appendChild(card);
      const target = sel === '.doc-card' ? card : card.querySelector(sel);
      hover(target);
      await vi.advanceTimersByTimeAsync(300);
      expect(tooltipEl()).toBeNull();
      expect(api.fetchFileContent).not.toHaveBeenCalled();
    },
  );

  it('DocCard：悬停 .badges 显示浮层；主题/标题/时间/整卡不触发', async () => {
    const cp = 'proj/topic/202605181220-doccard.md';
    api.fetchFileContent.mockResolvedValue('# **DocCard** digest');
    const card = renderDocCard('dc1', makeEntry(cp), 'Title');
    const badges = card.querySelector('.badges');

    hover(card.querySelector('.doc-topic'));
    hover(card.querySelector('.doc-title-btn'));
    hover(card.querySelector('.doc-meta'));
    hover(card);
    await vi.advanceTimersByTimeAsync(300);
    expect(tooltipEl()).toBeNull();
    expect(api.fetchFileContent).not.toHaveBeenCalled();

    hover(badges);
    await vi.advanceTimersByTimeAsync(300);
    const tip = tooltipEl();
    expect(tip).not.toBeNull();
    expect(tip.textContent).toBe('DocCard digest');
    expect(api.fetchFileContent).toHaveBeenCalledWith('digest', cp);
  });

  it('buildCard 没有 .badges：不回退挂到整张卡片', async () => {
    const cp = 'proj/topic/202605181230-nobadges.md';
    api.fetchFileContent.mockResolvedValue('# digest');
    const qs = hideBadgesQuery();
    const card = buildCard('nb1', makeEntry(cp), 'Title');
    qs.mockRestore();
    document.getElementById('doc-list').appendChild(card);

    hover(card);
    await vi.advanceTimersByTimeAsync(300);
    expect(tooltipEl()).toBeNull();
    expect(api.fetchFileContent).not.toHaveBeenCalled();
  });

  it('DocCard 没有 .badges：不回退挂到整张卡片', async () => {
    const cp = 'proj/topic/202605181240-nobadges-dc.md';
    api.fetchFileContent.mockResolvedValue('# digest');
    const qs = hideBadgesQuery();
    const card = renderDocCard('nb2', makeEntry(cp), 'Title');
    qs.mockRestore();

    hover(card);
    await vi.advanceTimersByTimeAsync(300);
    expect(tooltipEl()).toBeNull();
    expect(api.fetchFileContent).not.toHaveBeenCalled();
  });
});

describe('card open-in-chat sits next to the star', () => {
  const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

  function makeEntry(overrides = {}) {
    return {
      common_path: 'proj/topic/202605181200-note.md',
      created_at: '202605181200',
      layers: ['raw'],
      source_type: 'summary',
      ...overrides,
    };
  }

  beforeEach(() => {
    vi.mocked(openNoteInChat).mockClear();
  });

  it('buildCard places the chat icon immediately after importance', () => {
    const card = buildCard('chat1', makeEntry(), 'Title');
    const importance = card.querySelector('.badge-importance');
    const chat = card.querySelector('.badge-open-in-chat');
    expect(importance).not.toBeNull();
    expect(chat).not.toBeNull();
    expect(importance.nextElementSibling).toBe(chat);
    expect(chat.getAttribute('aria-label')).toBe('Open in chat');
    expect(chat.querySelector('[data-viewer-icon="chat"]')).not.toBeNull();
  });

  it('clicking the chat badge opens chat and does not open the note', () => {
    const opened = [];
    const onOpen = (e) => opened.push(e.detail);
    document.addEventListener('cta:open-entry', onOpen);
    const card = buildCard('chat2', makeEntry(), 'Title');
    document.getElementById('doc-list').appendChild(card);
    card.querySelector('[data-action="open-in-chat"]').click();
    document.removeEventListener('cta:open-entry', onOpen);
    expect(openNoteInChat).toHaveBeenCalledTimes(1);
    expect(openNoteInChat).toHaveBeenCalledWith(
      expect.objectContaining({ _id: 'chat2', common_path: 'proj/topic/202605181200-note.md' }),
      expect.any(HTMLButtonElement),
    );
    expect(opened).toEqual([]);
  });

  it('DocCard chat badge calls openNoteInChat', () => {
    const host = document.createElement('div');
    document.getElementById('doc-list').appendChild(host);
    const root = createRoot(host);
    flushSync(() => {
      root.render(createElement(DocCard, { id: 'chat4', entry: makeEntry(), title: 'Title' }));
    });
    host.querySelector('[data-action="open-in-chat"]').click();
    expect(openNoteInChat).toHaveBeenCalledWith(
      expect.objectContaining({ _id: 'chat4' }),
      expect.any(HTMLButtonElement),
    );
    flushSync(() => root.unmount());
  });

  it('updateTitlesInDOM keeps the chat badge after the star', () => {
    const entry = makeEntry({ source_type: 'dialogue' });
    const card = buildCard('chat3', entry, null);
    document.getElementById('doc-list').appendChild(card);
    state.index.titleCache.set('20260518', new Map([['chat3', 'Loaded Title']]));
    state.index.groupedByDate = [{ date: '20260518', entries: [{ id: 'chat3', entry }] }];
    updateTitlesInDOM('20260518');
    const importance = card.querySelector('.badge-importance');
    const chat = card.querySelector('.badge-open-in-chat');
    expect(importance.nextElementSibling).toBe(chat);
  });

  it('star and chat match the Summary badge height and sit tightly', () => {
    const css = readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');
    expect(css).toMatch(/\.doc-card-icon-actions\s*\{[^}]*gap:\s*2px/);
    expect(css).toMatch(/\.badge-importance-unset,\s*\n\s*\.badge-open-in-chat\s*\{[^}]*min-height:\s*18px/);
    expect(css).toMatch(
      /\.badge-open-in-chat\s+\.viewer-header-icon\s*\{[^}]*width:\s*12px[^}]*height:\s*12px/,
    );
  });
});
