import { fetchFileContent } from '../../host/api.ts';
import { TIP_DELAY_MS, placeTip } from '../../shared/tooltip/tooltip.ts';

const PREVIEW_MAX_CHARS = 400;
const DIGEST_HEADER_RE = /^#\s+[^\n]+\n+>\s*创建时间：[^\n]+\n+(?:---\s*\n+)?/;
const INTERACTIVE_SEL = 'button, [data-action], [data-tag-key], .badge-links, [data-tip]';

function isInteractiveTarget(target: EventTarget | null): boolean {
  return target instanceof Element && Boolean(target.closest(INTERACTIVE_SEL));
}

/** 模块级会话内缓存：commonPath → digest 原文；null = 已确认无 digest、不再请求 */
export const digestCache = new Map<string, string | null>();

export function digestPreviewText(raw: string): string {
  const body = String(raw ?? '').replace(DIGEST_HEADER_RE, '');
  const plain = body
    .replace(/^#{1,6}\s+/gm, '')
    .replace(/[*_`]/g, '')
    .trim();
  if (plain.length <= PREVIEW_MAX_CHARS) return plain;
  return plain.slice(0, PREVIEW_MAX_CHARS).trimEnd() + '…';
}

function buildTooltip(text: string): HTMLDivElement {
  const tip = document.createElement('div');
  tip.className = 'ui-tip ui-tip-preview';
  tip.textContent = text;
  return tip;
}

export function attachDigestTooltip(el: HTMLElement, id: string): () => void {
  let hoverSession = 0;
  let showTimer: ReturnType<typeof setTimeout> | null = null;
  let tooltip: HTMLDivElement | null = null;

  function removeTooltip() {
    if (showTimer) {
      clearTimeout(showTimer);
      showTimer = null;
    }
    if (tooltip) {
      tooltip.remove();
      tooltip = null;
    }
  }

  async function show(session: number) {
    let content = digestCache.get(id);
    if (content === undefined) {
      try {
        const text = await fetchFileContent('digest', id);
        content = typeof text === 'string' && text.trim() ? text : null;
      } catch (err) {
        console.warn('digest tooltip: fetch digest failed for', id, err);
        content = null;
      }
      digestCache.set(id, content);
    }
    if (session !== hoverSession) return;
    if (content === null) return;
    if (!el.isConnected) return;
    removeTooltip();
    tooltip = buildTooltip(digestPreviewText(content));
    document.body.appendChild(tooltip);
    placeTip(tooltip, el.getBoundingClientRect());
  }

  let suppressShow = false;

  function scheduleShow() {
    if (showTimer || tooltip) return;
    const session = ++hoverSession;
    showTimer = setTimeout(() => void show(session), TIP_DELAY_MS);
  }

  function onMouseOver(e: MouseEvent) {
    if (isInteractiveTarget(e.target)) {
      suppressShow = true;
      return;
    }
    suppressShow = false;
    scheduleShow();
  }

  function onMouseEnter(e: MouseEvent) {
    if (suppressShow || isInteractiveTarget(e.target)) return;
    scheduleShow();
  }

  function onMouseLeave() {
    suppressShow = false;
    hoverSession++;
    removeTooltip();
  }

  el.addEventListener('mouseover', onMouseOver);
  el.addEventListener('mouseenter', onMouseEnter);
  el.addEventListener('mouseleave', onMouseLeave);

  return function detach() {
    hoverSession++;
    removeTooltip();
    el.removeEventListener('mouseover', onMouseOver);
    el.removeEventListener('mouseenter', onMouseEnter);
    el.removeEventListener('mouseleave', onMouseLeave);
  };
}
