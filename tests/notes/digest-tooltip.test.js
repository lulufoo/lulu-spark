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
  el.dispatchEvent(new MouseEvent('mouseenter'));
}

function leave(el) {
  el.dispatchEvent(new MouseEvent('mouseleave'));
}

function tooltipEl() {
  return document.querySelector('.digest-tooltip');
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
    expect(tip.textContent).toBe('Default digest');
    expect(tip.style.maxWidth).toBe('360px');
    expect(tip.style.whiteSpace).toBe('pre-wrap');
    // 锚点下方左对齐
    expect(parseFloat(tip.style.left)).toBe(100);
    expect(parseFloat(tip.style.top)).toBeGreaterThanOrEqual(120);
    expect(parseFloat(tip.style.top) - 120).toBeLessThanOrEqual(20);
    rectSpy.mockRestore();
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
