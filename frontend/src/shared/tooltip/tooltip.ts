/**
 * The one hover tooltip. Put `data-tip="text"` on any element; this module shows it.
 *
 * Look: the `.ui-tip` class in app.css (copied from the Stage copy-path tip).
 * Timing: `TIP_DELAY_MS`. Position: next to the element, below it, flipped above when it does not fit.
 * Add `data-tip-truncated` to a text element that ends in "…": its tip then only shows when the text is cut off.
 * Do not use the native `title` attribute for hover hints (slow, system style); a gate test enforces that.
 */

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

/** Start listening for `data-tip` hovers. Returns a function that stops and removes the tip (for tests). */
export function initTooltip(): () => void {
  const box = document.createElement('div');
  box.className = 'ui-tip';
  box.dataset.role = 'tip';
  box.setAttribute('role', 'tooltip');
  box.style.display = 'none';
  document.body.appendChild(box);

  let current: HTMLElement | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;

  function hide() {
    if (timer != null) clearTimeout(timer);
    timer = null;
    current = null;
    box.style.display = 'none';
  }

  function show(el: HTMLElement) {
    if (el !== current || !el.isConnected) return;
    const text = el.dataset.tip;
    if (!text) return;
    if (el.hasAttribute('data-tip-truncated') && !isCutOff(el)) return;
    box.textContent = text;
    box.style.display = 'block';
    placeTip(box, el.getBoundingClientRect());
  }

  function onOver(e: Event) {
    const el = (e.target as Element | null)?.closest<HTMLElement>('[data-tip]') ?? null;
    if (el === current) return;
    hide();
    if (!el || !el.dataset.tip) return;
    current = el;
    timer = setTimeout(() => show(el), TIP_DELAY_MS);
  }

  function onOut(e: MouseEvent) {
    if (!current) return;
    const next = e.relatedTarget as Node | null;
    if (next && current.contains(next)) return;
    hide();
  }

  const listeners: [string, EventListener, boolean | AddEventListenerOptions][] = [
    ['mouseover', onOver, true],
    ['mouseout', onOut as EventListener, true],
    ['mousedown', hide, true],
    ['keydown', hide, true],
    ['scroll', hide, true],
    ['blur', hide, false],
  ];
  const target = (name: string) => (name === 'blur' ? window : document);
  for (const [name, fn, opt] of listeners) target(name).addEventListener(name, fn, opt);

  return () => {
    hide();
    for (const [name, fn, opt] of listeners) target(name).removeEventListener(name, fn, opt);
    box.remove();
  };
}
