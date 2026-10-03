// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { digestPreviewText, attachDigestTooltip, digestCache } from '../../frontend/src/notes/ui/digest-tooltip.tsx';
import * as api from '../../frontend/src/host/api.ts';

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchFileContent: vi.fn(),
}));

const CP = 'proj/topic/202605181200-note.md';

function anchor() {
  const el = document.createElement('div');
  el.textContent = 'card';
  document.body.appendChild(el);
  return el;
}

function hover(el) {
  el.dispatchEvent(new MouseEvent('mouseover', { bubbles: true }));
  el.dispatchEvent(new MouseEvent('mouseenter'));
}

function leave(el) {
  el.dispatchEvent(new MouseEvent('mouseout', { bubbles: true }));
  el.dispatchEvent(new MouseEvent('mouseleave'));
}

function badgeRow() {
  const row = document.createElement('div');
  row.className = 'badges';

  const source = document.createElement('span');
  source.className = 'badge badge-source';
  source.textContent = 'Note';

  const tagBtn = document.createElement('button');
  tagBtn.type = 'button';
  tagBtn.className = 'badge badge-tag';
  tagBtn.dataset.tagKey = 'foo';
  tagBtn.textContent = 'tag';

  const links = document.createElement('span');
  links.className = 'badge badge-links';
  const linksInner = document.createElement('span');
  linksInner.textContent = '👍 ×1';
  links.appendChild(linksInner);

  const actionBtn = document.createElement('button');
  actionBtn.className = 'badge badge-done';
  actionBtn.dataset.action = 'toggle-done';
  const actionInner = document.createElement('span');
  actionInner.textContent = '○ Mark done';
  actionBtn.appendChild(actionInner);

  row.append(source, tagBtn, links, actionBtn);
  document.body.appendChild(row);
  return { row, source, tagBtn, links, linksInner, actionBtn, actionInner };
}

/** Enter the row as if the pointer landed on `target` (mouseover bubbles; mouseenter does not). */
function hoverFrom(row, target) {
  target.dispatchEvent(new MouseEvent('mouseover', { bubbles: true }));
  row.dispatchEvent(new MouseEvent('mouseenter'));
}

function tooltipEl() {
  return document.querySelector('.digest-tooltip');
}

function cssColor(value) {
  const probe = document.createElement('div');
  probe.style.color = value;
  return probe.style.color;
}

function cssBackground(value) {
  const probe = document.createElement('div');
  probe.style.background = value;
  return probe.style.background;
}

function cssBorder(value) {
  const probe = document.createElement('div');
  probe.style.border = value;
  return probe.style.border;
}

beforeEach(() => {
  vi.useFakeTimers();
  api.fetchFileContent.mockReset();
  api.fetchFileContent.mockResolvedValue('# Default digest');
  document.body.innerHTML = '';
  digestCache.clear();
});

afterEach(() => {
  vi.useRealTimers();
});

describe('digestPreviewText', () => {
  it('剥离 markdown 标记：标题井号与 *_` 字符', () => {
    expect(digestPreviewText('# Title\n\n## Sub\n\n**bold** _em_ `code`')).toBe(
      'Title\n\nSub\n\nbold em code',
    );
  });

  it('文首一级标题、创建时间与 --- 之后只保留正文', () => {
    const raw = '# Title\n\n> 创建时间：2026年6月19日 14:30\n\n---\n\nHello **world**.\n';
    expect(digestPreviewText(raw)).toBe('Hello world.');
  });

  it('文首一级标题与创建时间、无 --- 时从正文起算', () => {
    const raw = '# Test — 摘要\n\n> 创建时间：2026年6月19日 14:30\n\n## 概述\n\noverview';
    expect(digestPreviewText(raw)).toBe('概述\n\noverview');
  });

  it('短文本原样返回（仅 trim），不加省略号', () => {
    expect(digestPreviewText('  short text  ')).toBe('short text');
  });

  it('抽完后正文恰好 400 字原样返回，不加省略号', () => {
    expect(digestPreviewText('a'.repeat(400))).toBe('a'.repeat(400));
  });

  it('约 400 字符截断加省略号', () => {
    expect(digestPreviewText('a'.repeat(450))).toBe('a'.repeat(400) + '…');
  });

  it('没有文首一级标题、创建时间或 --- 时整篇当正文再截断', () => {
    expect(digestPreviewText('just a paragraph')).toBe('just a paragraph');
    expect(digestPreviewText('# Title\n\n## Sub\n\nplain ' + 'c'.repeat(400))).toBe(
      ('Title\n\nSub\n\nplain ' + 'c'.repeat(400)).slice(0, 400).trimEnd() + '…',
    );
  });

  it('空字符串或只含空白返回空字符串', () => {
    expect(digestPreviewText('')).toBe('');
    expect(digestPreviewText('   \n\t  ')).toBe('');
  });
});

describe('attachDigestTooltip 正常分支：digest 存在', () => {
  it('mouseenter 300ms 后显示纯文本预览：markdown 剥离、样式与锚点下方左对齐', async () => {
    Object.defineProperty(window, 'innerHeight', { value: 768, configurable: true });
    const rectSpy = vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function () {
      if (this.classList && this.classList.contains('digest-tooltip')) {
        return { x: 0, y: 0, top: 500, bottom: 540, left: 100, right: 460, width: 360, height: 40, toJSON() {} };
      }
      return { x: 100, y: 100, top: 100, bottom: 120, left: 100, right: 300, width: 200, height: 20, toJSON() {} };
    });

    const el = anchor();
    attachDigestTooltip(el, CP);

    hover(el);
    await vi.advanceTimersByTimeAsync(299);
    expect(tooltipEl()).toBeNull();
    expect(api.fetchFileContent).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(1);
    expect(api.fetchFileContent).toHaveBeenCalledWith('digest', CP);

    const tip = tooltipEl();
    expect(tip).not.toBeNull();
    expect(tip.id).not.toBe('_tip');
    expect(document.getElementById('_tip')).toBeNull();
    expect(tip.className).toBe('digest-tooltip');
    expect(tip.textContent).toBe('Default digest');
    expect(tip.style.maxWidth).toBe('360px');
    expect(tip.style.whiteSpace).toBe('pre-wrap');
    expect(tip.style.background).toBe(cssBackground('#fff'));
    expect(tip.style.color).toBe(cssColor('#24292f'));
    expect(tip.style.border).toBe(cssBorder('1px solid #d0d7de'));
    expect(tip.style.boxShadow).toBe('0 8px 24px rgba(27, 31, 36, 0.12)');
    expect(tip.style.background).not.toMatch(/--bg-tooltip|#2a2a2e/);
    expect(tip.style.color).not.toMatch(/--text-tooltip|#f0f0f0/);
    // 锚点下方左对齐
    expect(parseFloat(tip.style.left)).toBe(100);
    expect(parseFloat(tip.style.top)).toBeGreaterThanOrEqual(120);
    expect(parseFloat(tip.style.top) - 120).toBeLessThanOrEqual(20);
    rectSpy.mockRestore();
  });

  it('预览文本走 digestPreviewText：文首标题与创建时间不进浮层', async () => {
    api.fetchFileContent.mockResolvedValue(
      '# Title\n\n> 创建时间：2026年6月19日 14:30\n\n---\n\nHello **world**.\n',
    );
    const el = anchor();
    attachDigestTooltip(el, CP);
    hover(el);
    await vi.advanceTimersByTimeAsync(300);
    expect(tooltipEl().textContent).toBe('Hello world.');
  });

  it('长 digest 组件级截断：textContent 为 400 字符加省略号', async () => {
    api.fetchFileContent.mockResolvedValue('# ' + 'b'.repeat(460));
    const el = anchor();
    attachDigestTooltip(el, CP);
    hover(el);
    await vi.advanceTimersByTimeAsync(300);
    const tip = tooltipEl();
    expect(tip.textContent).toBe('b'.repeat(400) + '…');
  });
});

describe('attachDigestTooltip 边界分支：徽章行可点子控件不触发', () => {
  it.each([
    ['button', (row) => row.tagBtn],
    ['[data-action]', (row) => row.actionBtn],
    ['[data-tag-key]', (row) => row.tagBtn],
    ['.badge-links', (row) => row.links],
    ['button 后代', (row) => row.actionInner],
    ['.badge-links 后代', (row) => row.linksInner],
  ])('mouseover %s：不显示浮层、不发 digest 请求', async (_label, pick) => {
    const parts = badgeRow();
    attachDigestTooltip(parts.row, CP);
    hoverFrom(parts.row, pick(parts));
    await vi.advanceTimersByTimeAsync(300);
    expect(tooltipEl()).toBeNull();
    expect(api.fetchFileContent).not.toHaveBeenCalled();
  });

  it('锚点为徽章行且目标不是可点控件：300ms 后浅色独立浮层', async () => {
    const parts = badgeRow();
    attachDigestTooltip(parts.row, CP);
    hoverFrom(parts.row, parts.source);
    await vi.advanceTimersByTimeAsync(299);
    expect(tooltipEl()).toBeNull();
    await vi.advanceTimersByTimeAsync(1);
    const tip = tooltipEl();
    expect(tip).not.toBeNull();
    expect(tip.className).toBe('digest-tooltip');
    expect(document.getElementById('_tip')).toBeNull();
    expect(tip.style.background).toBe(cssBackground('#fff'));
    expect(tip.style.color).toBe(cssColor('#24292f'));
    expect(tip.style.border).toBe(cssBorder('1px solid #d0d7de'));
    expect(api.fetchFileContent).toHaveBeenCalledWith('digest', CP);
  });

  it('先划过可点控件再划到行内不可点区域：才发一次 digest 请求', async () => {
    const parts = badgeRow();
    attachDigestTooltip(parts.row, CP);
    hoverFrom(parts.row, parts.actionBtn);
    await vi.advanceTimersByTimeAsync(300);
    expect(api.fetchFileContent).not.toHaveBeenCalled();

    leave(parts.row);
    hoverFrom(parts.row, parts.source);
    await vi.advanceTimersByTimeAsync(300);
    expect(api.fetchFileContent).toHaveBeenCalledTimes(1);
    expect(tooltipEl()).not.toBeNull();
  });
});

describe('attachDigestTooltip 边界分支：视同无 digest', () => {
  it('digest 内容为空字符串：不显示、缓存 null、再次悬停不再请求', async () => {
    api.fetchFileContent.mockResolvedValue('');
    const el = anchor();
    attachDigestTooltip(el, CP);

    hover(el);
    await vi.advanceTimersByTimeAsync(300);
    expect(tooltipEl()).toBeNull();
    expect(digestCache.get(CP)).toBeNull();

    leave(el);
    hover(el);
    await vi.advanceTimersByTimeAsync(300);
    expect(tooltipEl()).toBeNull();
    expect(api.fetchFileContent).toHaveBeenCalledTimes(1);
  });

  it('请求失败：console.warn、视同无 digest、缓存 null 不再请求', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    api.fetchFileContent.mockRejectedValueOnce(new Error('boom'));
    const el = anchor();
    attachDigestTooltip(el, CP);

    hover(el);
    await vi.advanceTimersByTimeAsync(300);
    expect(tooltipEl()).toBeNull();
    expect(warn).toHaveBeenCalled();
    expect(digestCache.get(CP)).toBeNull();

    leave(el);
    hover(el);
    await vi.advanceTimersByTimeAsync(300);
    expect(api.fetchFileContent).toHaveBeenCalledTimes(1);
    warn.mockRestore();
  });

  it('同一 commonPath 第二次悬停命中模块级缓存不再发请求', async () => {
    const el = anchor();
    attachDigestTooltip(el, CP);

    hover(el);
    await vi.advanceTimersByTimeAsync(300);
    expect(tooltipEl()).not.toBeNull();
    expect(digestCache.get(CP)).toBe('# Default digest');

    leave(el);
    expect(tooltipEl()).toBeNull();

    hover(el);
    await vi.advanceTimersByTimeAsync(300);
    expect(api.fetchFileContent).toHaveBeenCalledTimes(1);
    expect(tooltipEl().textContent).toBe('Default digest');
  });

  it('锚点下方空间不足时 tooltip 翻转到上方', async () => {
    Object.defineProperty(window, 'innerHeight', { value: 768, configurable: true });
    const rectSpy = vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function () {
      if (this.classList && this.classList.contains('digest-tooltip')) {
        return { x: 0, y: 0, top: 500, bottom: 540, left: 100, right: 460, width: 360, height: 40, toJSON() {} };
      }
      // 锚点贴近视口底部：下方剩余 18px < tooltip 高 40 + 间距
      return { x: 100, y: 730, top: 730, bottom: 750, left: 100, right: 300, width: 200, height: 20, toJSON() {} };
    });

    const el = anchor();
    attachDigestTooltip(el, CP);
    hover(el);
    await vi.advanceTimersByTimeAsync(300);

    const tip = tooltipEl();
    expect(tip).not.toBeNull();
    expect(parseFloat(tip.style.left)).toBe(100);
    // 翻转到锚点上方：tooltip 顶边在锚点 top 之上、但不越过锚点上方 40 高度加间距
    expect(parseFloat(tip.style.top)).toBeLessThanOrEqual(730);
    expect(parseFloat(tip.style.top)).toBeGreaterThanOrEqual(730 - 40 - 20);
    rectSpy.mockRestore();
  });
});

describe('attachDigestTooltip 异常分支：关闭与解绑', () => {
  it('mouseleave 立即关闭并移除 tooltip 节点', async () => {
    const el = anchor();
    attachDigestTooltip(el, CP);
    hover(el);
    await vi.advanceTimersByTimeAsync(300);
    expect(tooltipEl()).not.toBeNull();

    leave(el);
    expect(tooltipEl()).toBeNull();
  });

  it('300ms 延迟内 mouseleave：不触发请求也不显示', async () => {
    const el = anchor();
    attachDigestTooltip(el, CP);
    hover(el);
    await vi.advanceTimersByTimeAsync(100);
    leave(el);
    await vi.advanceTimersByTimeAsync(500);
    expect(api.fetchFileContent).not.toHaveBeenCalled();
    expect(tooltipEl()).toBeNull();
  });

  it('懒加载进行中不阻塞调用方：无 loading 态、同步返回解绑函数', async () => {
    api.fetchFileContent.mockImplementationOnce(() => new Promise(() => {}));
    const el = anchor();
    const detach = attachDigestTooltip(el, CP);
    expect(typeof detach).toBe('function');

    hover(el);
    await vi.advanceTimersByTimeAsync(300);
    expect(tooltipEl()).toBeNull();
  });

  it('加载进行中 mouseleave：请求完成也不显示', async () => {
    let resolveFetch;
    api.fetchFileContent.mockImplementationOnce(() => new Promise((res) => { resolveFetch = res; }));
    const el = anchor();
    attachDigestTooltip(el, CP);
    hover(el);
    await vi.advanceTimersByTimeAsync(300);
    expect(tooltipEl()).toBeNull();

    leave(el);
    resolveFetch('# Late digest');
    await vi.advanceTimersByTimeAsync(0);
    expect(tooltipEl()).toBeNull();
  });

  it('解绑函数：移除已显示 tooltip 且后续 mouseenter 无反应', async () => {
    const el = anchor();
    const detach = attachDigestTooltip(el, CP);
    hover(el);
    await vi.advanceTimersByTimeAsync(300);
    expect(tooltipEl()).not.toBeNull();

    detach();
    expect(tooltipEl()).toBeNull();
    expect(digestCache.get(CP)).toBe('# Default digest');

    hover(el);
    await vi.advanceTimersByTimeAsync(300);
    expect(api.fetchFileContent).toHaveBeenCalledTimes(1);
    expect(tooltipEl()).toBeNull();
  });
});
