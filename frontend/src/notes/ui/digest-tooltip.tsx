import { fetchFileContent } from '../../host/api.ts';

const HOVER_DELAY_MS = 300;
const PREVIEW_MAX_CHARS = 200;
const TOOLTIP_MAX_WIDTH = '360px';
const TOOLTIP_GAP_PX = 6;

/** 模块级会话内缓存：commonPath → digest 原文；null = 已确认无 digest、不再请求 */
export const digestCache = new Map<string, string | null>();

export function digestPreviewText(raw: string): string {
  const plain = String(raw ?? '')
    .replace(/^#{1,6}\s+/gm, '')
    .replace(/[*_`]/g, '')
    .trim();
  if (plain.length <= PREVIEW_MAX_CHARS) return plain;
  return plain.slice(0, PREVIEW_MAX_CHARS).trimEnd() + '…';
}

function buildTooltip(text: string): HTMLDivElement {
  const tip = document.createElement('div');
  tip.className = 'digest-tooltip';
  tip.style.position = 'fixed';
  tip.style.maxWidth = TOOLTIP_MAX_WIDTH;
  tip.style.whiteSpace = 'pre-wrap';
  tip.style.zIndex = '1000';
  tip.style.padding = '6px 10px';
  tip.style.borderRadius = '6px';
  tip.style.background = 'var(--bg-tooltip, #2a2a2e)';
  tip.style.color = 'var(--text-tooltip, #f0f0f0)';
  tip.style.fontSize = '12px';
  tip.style.lineHeight = '1.5';
  tip.style.boxShadow = '0 2px 10px rgba(0, 0, 0, 0.25)';
  tip.textContent = text;
  return tip;
}

function placeTooltip(tip: HTMLDivElement, anchorRect: DOMRect) {
  const tipRect = tip.getBoundingClientRect();
  const spaceBelow = window.innerHeight - anchorRect.bottom;
  const flip = spaceBelow < tipRect.height + TOOLTIP_GAP_PX;
  tip.style.left = `${anchorRect.left}px`;
  tip.style.top = flip
    ? `${Math.max(anchorRect.top - tipRect.height - TOOLTIP_GAP_PX, 0)}px`
    : `${anchorRect.bottom + TOOLTIP_GAP_PX}px`;
}

export function attachDigestTooltip(el: HTMLElement, commonPath: string): () => void {
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
    let content = digestCache.get(commonPath);
    if (content === undefined) {
      try {
        const text = await fetchFileContent('digest', commonPath);
        content = typeof text === 'string' && text.trim() ? text : null;
      } catch (err) {
        console.warn('digest tooltip: fetch digest failed for', commonPath, err);
        content = null;
      }
      digestCache.set(commonPath, content);
    }
    if (session !== hoverSession) return;
    if (content === null) return;
    if (!el.isConnected) return;
    removeTooltip();
    tooltip = buildTooltip(digestPreviewText(content));
    document.body.appendChild(tooltip);
    placeTooltip(tooltip, el.getBoundingClientRect());
  }

  function onMouseEnter() {
    const session = ++hoverSession;
    showTimer = setTimeout(() => void show(session), HOVER_DELAY_MS);
  }

  function onMouseLeave() {
    hoverSession++;
    removeTooltip();
  }

  el.addEventListener('mouseenter', onMouseEnter);
  el.addEventListener('mouseleave', onMouseLeave);

  return function detach() {
    hoverSession++;
    removeTooltip();
    el.removeEventListener('mouseenter', onMouseEnter);
    el.removeEventListener('mouseleave', onMouseLeave);
  };
}
