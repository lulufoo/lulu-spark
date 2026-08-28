// @ts-nocheck — DOM wiring stays unchecked like checkJs:false.
const STORAGE_KEY = 'corpus_sidebar_width';
const DEFAULT_WIDTH = 360;
const MIN_WIDTH = 280;
const MAX_WIDTH = 560;

/** @type {(() => void) | null} */
let _cleanup = null;
/** @type {{ aside: HTMLElement, resizer: HTMLElement, startX: number, startWidth: number, lastWidth: number } | null} */
let _dragState = null;

function setCorpusSidebarWidth(aside, px) {
  const w = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, px));
  aside.style.setProperty('--corpus-sidebar-width', `${w}px`);
  aside.style.width = `${w}px`;
  aside.style.flexBasis = `${w}px`;
  return w;
}

function persistCorpusSidebarWidth(px) {
  localStorage.setItem(STORAGE_KEY, String(px));
}

function onPointerMove(e) {
  if (!_dragState) return;
  const next = setCorpusSidebarWidth(
    _dragState.aside,
    _dragState.startWidth + (e.clientX - _dragState.startX),
  );
  _dragState.lastWidth = next;
}

function onPointerUp(e) {
  if (!_dragState) return;
  const { resizer } = _dragState;
  if (
    typeof resizer.hasPointerCapture === 'function'
    && typeof resizer.releasePointerCapture === 'function'
    && resizer.hasPointerCapture(e.pointerId)
  ) {
    resizer.releasePointerCapture(e.pointerId);
  }
  document.removeEventListener('pointermove', onPointerMove);
  document.removeEventListener('pointerup', onPointerUp);
  document.body.classList.remove('sidebar-resizing');
  persistCorpusSidebarWidth(_dragState.lastWidth);
  _dragState = null;
}

/**
 * @param {HTMLElement} aside
 * @returns {() => void}
 */
export function attachCorpusSidebarResize(aside) {
  detachCorpusSidebarResize();
  if (!aside) return () => {};

  let resizer = aside.querySelector('.corpus-sidebar-resizer');
  if (!resizer) {
    resizer = document.createElement('div');
    resizer.className = 'corpus-sidebar-resizer sidebar-resizer';
    resizer.setAttribute('role', 'separator');
    resizer.setAttribute('aria-orientation', 'vertical');
    resizer.setAttribute('aria-label', 'Resize knowledge tree');
    resizer.tabIndex = 0;
    aside.appendChild(resizer);
  }

  const saved = parseInt(localStorage.getItem(STORAGE_KEY), 10);
  setCorpusSidebarWidth(aside, Number.isFinite(saved) ? saved : DEFAULT_WIDTH);

  const onPointerDown = (e) => {
    if (e.button !== 0) return;
    e.preventDefault();
    const startWidth = aside.getBoundingClientRect().width;
    _dragState = { aside, resizer, startX: e.clientX, startWidth, lastWidth: startWidth };
    if (typeof resizer.setPointerCapture === 'function') {
      resizer.setPointerCapture(e.pointerId);
    }
    document.body.classList.add('sidebar-resizing');
    document.addEventListener('pointermove', onPointerMove);
    document.addEventListener('pointerup', onPointerUp);
  };

  resizer.addEventListener('pointerdown', onPointerDown);

  _cleanup = () => {
    resizer.removeEventListener('pointerdown', onPointerDown);
    document.removeEventListener('pointermove', onPointerMove);
    document.removeEventListener('pointerup', onPointerUp);
    document.body.classList.remove('sidebar-resizing');
    _dragState = null;
  };

  return _cleanup;
}

export function detachCorpusSidebarResize() {
  _cleanup?.();
  _cleanup = null;
}

export { setCorpusSidebarWidth, STORAGE_KEY, DEFAULT_WIDTH, MIN_WIDTH, MAX_WIDTH };
