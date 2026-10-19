// @vitest-environment jsdom
import { TIP_DELAY_MS } from '../../frontend/src/shared/tooltip/tooltip.ts';
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
import { closeFloatingListSelect } from '../../frontend/src/shared/floating-list-select.ts';
import { state } from '../../frontend/src/host/state.ts';
import { setDocCategoryCatalog } from '../../frontend/src/host/doc-categories-catalog.ts';
import * as api from '../../frontend/src/host/api.ts';

vi.mock('../../frontend/src/host/api.ts', () => ({
  setImportance: vi.fn(),
  setDone: vi.fn(),
  setDocSourceType: vi.fn(async () => ({ ok: true })),
  fetchFileContent: vi.fn(),
}));

const BUILTIN_CATEGORIES = [
  { id: 'summary', label: 'Summary', exposed: true },
  { id: 'dialogue', label: 'Dialogue', exposed: true },
  { id: 'video', label: 'Video', exposed: true },
  { id: 'article', label: 'Article', exposed: true },
  { id: 'jot', label: 'Jot', exposed: false },
];

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
  setDocCategoryCatalog(BUILTIN_CATEGORIES);
});

afterEach(() => {
  closeFloatingListSelect();
});

describe('sourceTypeBadgeHtml', () => {
  it('dialogue 返回对话 badge HTML', () => {
    const html = sourceTypeBadgeHtml('dialogue');
    expect(html).toContain('badge-source-dialogue');
    expect(html).toContain('Dialogue');
    expect(html).toMatch(/^<button/);
  });

  it('article and video use catalog labels; unknown and theme-line have no badge', () => {
    expect(sourceTypeBadgeHtml('article')).toContain('Article');
    expect(sourceTypeBadgeHtml('video')).toContain('badge-source-video');
    expect(sourceTypeBadgeHtml('theme-line')).toBe('');
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
    expect(first.tagName).toBe('BUTTON');
    expect(first.dataset.action).toBe('set-source-type');
  });

  it('summary 显示黄色「总结」badge', () => {
    const entry = {
      project_id: 'proj',
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

  it('jot still shows a badge and the picker includes jot', async () => {
    const entry = {
      project_id: 'proj',
      created_at: '202605181200',
      layers: ['raw'],
      source_type: 'jot',
    };
    const card = buildCard('jot1', entry, 'Title');
    document.getElementById('doc-list').appendChild(card);
    const badge = card.querySelector('[data-action="set-source-type"]');
    expect(badge?.textContent).toBe('Jot');
    badge.click();
    const menu = document.querySelector('.list-select-menu');
    expect(menu).not.toBeNull();
    const values = [...menu.querySelectorAll('[data-value]')].map((el) => el.dataset.value);
    const labels = [...menu.querySelectorAll('[data-value]')].map((el) => el.textContent);
    expect(values).toEqual(['summary', 'dialogue', 'video', 'article', 'jot']);
    expect(labels).toEqual(['Summary', 'Dialogue', 'Video', 'Article', 'Jot']);
    expect(menu.style.width).toBe('max-content');
    expect(menu.style.minWidth).toBe(`${badge.getBoundingClientRect().width}px`);
    closeFloatingListSelect();
  });

  it('点 Dialogue badge 可改成 Summary，且不打开文档', async () => {
    const opened = [];
    const onOpen = (e) => opened.push(e.detail);
    document.addEventListener('cta:open-entry', onOpen);
    const entry = {
      project_id: 'proj',
      created_at: '202605181200',
      layers: ['raw'],
      source_type: 'dialogue',
    };
    const card = buildCard('id-change', entry, 'Title');
    document.getElementById('doc-list').appendChild(card);
    card.querySelector('[data-action="set-source-type"]').click();
    document.querySelector('[data-value="summary"]').click();
    await vi.waitFor(() => {
      expect(api.setDocSourceType).toHaveBeenCalledWith('id-change', 'summary');
    });
    expect(entry.source_type).toBe('summary');
    expect(opened).toEqual([]);
    document.removeEventListener('cta:open-entry', onOpen);
    closeFloatingListSelect();
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

describe('digest 小图标 hover 预览（buildCard / DocCard）', () => {
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
    return document.querySelector('.ui-tip-preview');
  }

  function makeEntry(commonPath, overrides = {}) {
    return {
      project_id: String(commonPath).split('/')[0],
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

  beforeEach(() => {
    vi.useFakeTimers();
    api.fetchFileContent.mockReset();
    digestCache.clear();
    vi.mocked(openNoteInChat).mockClear();
  });

  afterEach(() => {
    if (reactRoot) {
      flushSync(() => reactRoot.unmount());
      reactRoot = undefined;
    }
    vi.useRealTimers();
    document.querySelector('.ui-tip-preview')?.remove();
  });

  it('buildCard：有 digest 的卡片，图标在 Open in chat 左边、星标右边', () => {
    const card = buildCard('pos1', makeEntry('proj/topic/202605181199-pos.md'), 'Title');
    const icon = card.querySelector('.badge-digest');
    expect(icon).not.toBeNull();
    expect(icon.tagName).toBe('BUTTON');
    expect(icon.getAttribute('aria-label')).toBe('Digest preview');
    expect(icon.hasAttribute('data-tip')).toBe(false);
    expect(icon.querySelector('[data-viewer-icon="digest"]')).not.toBeNull();
    expect(card.querySelector('.badge-importance').nextElementSibling).toBe(icon);
    expect(icon.nextElementSibling).toBe(card.querySelector('.badge-open-in-chat'));
  });

  it('buildCard：没有 digest 的卡片没有图标，悬停徽章区也不请求', async () => {
    api.fetchFileContent.mockResolvedValue('# digest');
    const card = buildCard('nod1', makeEntry('proj/topic/202605181198-nod.md', { layers: ['raw'] }), 'Title');
    document.getElementById('doc-list').appendChild(card);
    expect(card.querySelector('.badge-digest')).toBeNull();

    hover(card.querySelector('.badges'));
    await vi.advanceTimersByTimeAsync(300);
    expect(tooltipEl()).toBeNull();
    expect(api.fetchFileContent).not.toHaveBeenCalled();
  });

  it('buildCard：悬停图标延迟后显示纯文本预览；移开收起；空 digest 无 DOM 反应', async () => {
    const cp = 'proj/topic/202605181200-tip.md';
    api.fetchFileContent.mockResolvedValueOnce('# **Plain** digest preview');
    const card = buildCard('tip1', makeEntry(cp), 'Title');
    document.getElementById('doc-list').appendChild(card);
    const badges = card.querySelector('.badge-digest');

    hover(badges);
    await vi.advanceTimersByTimeAsync(TIP_DELAY_MS - 1);
    expect(api.fetchFileContent).not.toHaveBeenCalled();
    expect(tooltipEl()).toBeNull();

    await vi.advanceTimersByTimeAsync(1);
    expect(api.fetchFileContent).toHaveBeenCalledWith('digest', 'tip1');
    const tip = tooltipEl();
    expect(tip).not.toBeNull();
    expect(tip.textContent).toBe('Plain digest preview');

    leave(badges);
    expect(tooltipEl()).toBeNull();

    const cp2 = 'proj/topic/202605181201-tip2.md';
    api.fetchFileContent.mockResolvedValueOnce('');
    const card2 = buildCard('tip2', makeEntry(cp2, { created_at: '202605181201' }), 'Title2');
    document.getElementById('doc-list').appendChild(card2);

    hover(card2.querySelector('.badge-digest'));
    await vi.advanceTimersByTimeAsync(300);
    expect(api.fetchFileContent).toHaveBeenLastCalledWith('digest', 'tip2');
    expect(tooltipEl()).toBeNull();
  });

  it('buildCard：点击图标什么都不发生（不弹出、不打开文档、不开聊天）', async () => {
    api.fetchFileContent.mockResolvedValue('# digest');
    const opened = [];
    const onOpen = (e) => opened.push(e.detail);
    document.addEventListener('cta:open-entry', onOpen);
    const card = buildCard('clk1', makeEntry('proj/topic/202605181202-clk.md'), 'Title');
    document.getElementById('doc-list').appendChild(card);

    card.querySelector('.badge-digest').click();
    await vi.advanceTimersByTimeAsync(300);
    document.removeEventListener('cta:open-entry', onOpen);

    expect(tooltipEl()).toBeNull();
    expect(api.fetchFileContent).not.toHaveBeenCalled();
    expect(opened).toEqual([]);
    expect(openNoteInChat).not.toHaveBeenCalled();
  });

  it.each(['.doc-card', '.doc-topic', '.doc-title-btn', '.doc-meta', '.badges', '.badge-done', '.badge-open-in-chat'])(
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

  it('DocCard：悬停图标显示浮层；其它位置不触发；点击图标无事发生', async () => {
    const cp = 'proj/topic/202605181220-doccard.md';
    api.fetchFileContent.mockResolvedValue('# **DocCard** digest');
    const opened = [];
    const onOpen = (e) => opened.push(e.detail);
    document.addEventListener('cta:open-entry', onOpen);
    const card = renderDocCard('dc1', makeEntry(cp), 'Title');

    hover(card.querySelector('.doc-topic'));
    hover(card.querySelector('.doc-title-btn'));
    hover(card.querySelector('.doc-meta'));
    hover(card.querySelector('.badges'));
    hover(card);
    await vi.advanceTimersByTimeAsync(300);
    expect(tooltipEl()).toBeNull();
    expect(api.fetchFileContent).not.toHaveBeenCalled();

    const icon = card.querySelector('.badge-digest');
    hover(icon);
    await vi.advanceTimersByTimeAsync(300);
    const tip = tooltipEl();
    expect(tip).not.toBeNull();
    expect(tip.textContent).toBe('DocCard digest');
    expect(api.fetchFileContent).toHaveBeenCalledWith('digest', 'dc1');

    leave(icon);
    expect(tooltipEl()).toBeNull();
    icon.click();
    await vi.advanceTimersByTimeAsync(300);
    document.removeEventListener('cta:open-entry', onOpen);
    expect(tooltipEl()).toBeNull();
    expect(opened).toEqual([]);
    expect(openNoteInChat).not.toHaveBeenCalled();
  });

  it('DocCard：没有 digest 没有图标；digest 出现后重渲染图标出现且可悬停', async () => {
    const cp = 'proj/topic/202605181230-later.md';
    api.fetchFileContent.mockResolvedValue('# later digest');
    const card = renderDocCard('lt1', makeEntry(cp, { layers: ['raw'] }), 'Title');
    expect(card.querySelector('.badge-digest')).toBeNull();

    flushSync(() => {
      reactRoot.render(createElement(DocCard, { id: 'lt1', entry: makeEntry(cp), title: 'Title' }));
    });
    const icon = card.querySelector('.badge-digest');
    expect(icon).not.toBeNull();

    hover(icon);
    await vi.advanceTimersByTimeAsync(300);
    expect(tooltipEl().textContent).toBe('later digest');
  });

  it('updateTitlesInDOM 重建徽章行后，新图标重新绑定 hover，旧浮层被清掉', async () => {
    const cp = 'proj/topic/202605181240-rebuild.md';
    api.fetchFileContent.mockResolvedValue('# rebuilt digest');
    const entry = makeEntry(cp);
    const card = buildCard('rb1', entry, null);
    document.getElementById('doc-list').appendChild(card);

    const oldIcon = card.querySelector('.badge-digest');
    hover(oldIcon);
    await vi.advanceTimersByTimeAsync(300);
    expect(tooltipEl()).not.toBeNull();

    state.index.titleCache.set('20260518', new Map([['rb1', 'Loaded Title']]));
    state.index.groupedByDate = [{ date: '20260518', entries: [{ id: 'rb1', entry }] }];
    updateTitlesInDOM('20260518');

    expect(tooltipEl()).toBeNull();
    const newIcon = card.querySelector('.badge-digest');
    expect(newIcon).not.toBeNull();
    expect(newIcon).not.toBe(oldIcon);

    hover(newIcon);
    await vi.advanceTimersByTimeAsync(300);
    expect(tooltipEl().textContent).toBe('rebuilt digest');
  });

  it('updateTitlesInDOM：没有 digest 的卡片重建后仍没有图标', () => {
    const entry = makeEntry('proj/topic/202605181250-nod2.md', { layers: ['raw'] });
    const card = buildCard('rb2', entry, null);
    document.getElementById('doc-list').appendChild(card);
    state.index.titleCache.set('20260518', new Map([['rb2', 'T']]));
    state.index.groupedByDate = [{ date: '20260518', entries: [{ id: 'rb2', entry }] }];
    updateTitlesInDOM('20260518');
    expect(card.querySelector('.badge-digest')).toBeNull();
  });
});

describe('card open-in-chat sits next to the star', () => {
  const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

  function makeEntry(overrides = {}) {
    return {
      project_id: 'proj',
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
      expect.objectContaining({ _id: 'chat2', project_id: 'proj' }),
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

  it('digest icon shares the chat icon size, color and hover style', () => {
    const css = readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');
    expect(css).toMatch(/\.badge-digest,\s*\n\s*\.badge-importance-unset,\s*\n\s*\.badge-open-in-chat\s*\{[^}]*min-height:\s*18px/);
    expect(css).toMatch(/\.badge-digest,\s*\n\s*\.badge-open-in-chat\s*\{[^}]*cursor:\s*pointer[^}]*background:\s*none/);
    expect(css).toMatch(
      /\.badge-digest\s+\.viewer-header-icon,\s*\n\s*\.badge-open-in-chat\s+\.viewer-header-icon\s*\{[^}]*width:\s*12px[^}]*height:\s*12px/,
    );
    expect(css).toMatch(/\.badge-digest:hover,\s*\n\s*\.badge-open-in-chat:hover\s*\{/);
  });
});
