/**
 * The one hover tooltip. Put `data-tip="text"` on any element; this module shows it.
 *
 * Look: `.ui-tip` in app.css, or the comment popover for highlight previews.
 * Timing: `TIP_DELAY_MS`. Position: next to the element, below it, flipped above when it does not fit.
 * Add `data-tip-truncated` to a text element that ends in "…": its tip then only shows when the text is cut off.
 * Do not use the native `title` attribute for hover hints (slow, system style); a gate test enforces that.
 */

import { highlightCommentFragment, placeHighlightCommentPopover } from './highlight-comment-popover.ts';

/** Pointer must rest on the element this long before the tip shows (every tip in the app shares it). */
export const TIP_DELAY_MS = 100;
/** Space between the element and its tip. */
export const TIP_GAP_PX = 6;
/** Keep the tip this far from the window edges. */
const EDGE_PX = 4;

/** Put `tip` (already in the DOM and measurable) below `anchor`, or above it when there is no room. */
export function placeTip(tip: HTMLElement, anchor: DOMRect): void {
  const { width, height } = tip.getBoundingClientRect();
  const below = anchor.bottom + TIP_GAP_PX;
  const top = below + height > window.innerHeight - EDGE_PX
    ? Math.max(EDGE_PX, anchor.top - height - TIP_GAP_PX)
    : below;
  const maxLeft = Math.max(EDGE_PX, window.innerWidth - width - EDGE_PX);
  const left = Math.min(Math.max(EDGE_PX, anchor.left), maxLeft);
  tip.style.left = `${left}px`;
  tip.style.top = `${top}px`;
}

function isCutOff(el: HTMLElement): boolean {
  return el.scrollWidth > el.clientWidth;
}

function renderHighlightComment(box: HTMLElement, text: string): void {
  box.className = 'highlight-comment-popover highlight-comment-preview';
  const content = document.createElement('div');
  content.className = 'highlight-comment-preview__content';
  const title = document.createElement('h3');
  title.textContent = 'Highlight comment';
  const body = document.createElement('p');
  body.className = 'highlight-comment-preview__text';
  body.textContent = text;
  content.append(title, body);
  box.replaceChildren(content);
}

/** Start listening for `data-tip` hovers. Returns a function that stops and removes the tip (for tests). */
export function initTooltip(): () => void {
  const box = document.createElement('div');
  box.className = 'ui-tip';
  box.dataset.role = 'tip';
  box.setAttribute('role', 'tooltip');
  box.style.display = 'none';
  document.body.appendChild(box);

  let current: HTMLElement | null = null;
  let displayedText: string | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;
  const observer = new MutationObserver(() => {
    if (current && (!current.isConnected || current.dataset.tip !== displayedText)) hide();
  });

  function hide() {
    if (timer != null) clearTimeout(timer);
    timer = null;
    current = null;
    displayedText = null;
    observer.disconnect();
    box.style.display = 'none';
    box.className = 'ui-tip';
  }

  function show(el: HTMLElement, event: MouseEvent) {
    if (el !== current || !el.isConnected) return;
    const text = el.dataset.tip;
    if (!text) return;
    if (el.hasAttribute('data-tip-truncated') && !isCutOff(el)) return;
    const isHighlightComment = el.dataset.tipVariant === 'highlight-comment';
    if (isHighlightComment && document.querySelector('.highlight-comment-editor, #comment-delete-dialog.open')) return;
    if (isHighlightComment) renderHighlightComment(box, text);
    else box.textContent = text;
    displayedText = text;
    box.style.display = 'block';
    if (isHighlightComment) {
      placeHighlightCommentPopover(box, el, highlightCommentFragment(el, event));
    } else {
      placeTip(box, el.getBoundingClientRect());
    }
    observer.observe(document.body, { childList: true, subtree: true });
    observer.observe(el, { attributes: true, attributeFilter: ['data-tip'] });
  }

  function onOver(e: MouseEvent) {
    const el = (e.target as Element | null)?.closest<HTMLElement>('[data-tip]') ?? null;
    if (el === current) return;
    hide();
    if (!el || !el.dataset.tip) return;
    current = el;
    timer = setTimeout(() => show(el, e), TIP_DELAY_MS);
  }

  function onOut(e: MouseEvent) {
    if (!current) return;
    const next = e.relatedTarget as Node | null;
    if (next && current.contains(next)) return;
    hide();
  }

  const listeners: [string, EventListener, boolean | AddEventListenerOptions][] = [
    ['mouseover', onOver as EventListener, true],
    ['mouseout', onOut as EventListener, true],
    ['mousedown', hide, true],
    ['click', hide, true],
    ['keydown', hide, true],
    ['scroll', hide, true],
    ['resize', hide, false],
    ['blur', hide, false],
  ];
  const target = (name: string) => (name === 'blur' || name === 'resize' ? window : document);
  for (const [name, fn, opt] of listeners) target(name).addEventListener(name, fn, opt);

  return () => {
    hide();
    for (const [name, fn, opt] of listeners) target(name).removeEventListener(name, fn, opt);
    box.remove();
  };
}
