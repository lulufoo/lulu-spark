const STORAGE_KEY = 'cta_sidebar_width';
const DEFAULT_WIDTH = 221;
const MIN_WIDTH = 180;
const MAX_WIDTH = 400;

type DragState = {
  aside: HTMLElement;
  resizer: HTMLElement;
  startX: number;
  startWidth: number;
  lastWidth: number;
};

let _cleanup: (() => void) | null = null;
let _dragState: DragState | null = null;

function setSidebarWidth(aside: HTMLElement, px: number) {
  const w = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, px));
  aside.style.setProperty('--sidebar-width', `${w}px`);
  aside.style.width = `${w}px`;
  aside.style.flexBasis = `${w}px`;
  return w;
}

function persistSidebarWidth(px: number) {
  globalThis.localStorage?.setItem?.(STORAGE_KEY, String(px));
}

function onPointerMove(e: PointerEvent) {
  if (!_dragState) return;
  const next = setSidebarWidth(_dragState.aside, _dragState.startWidth + (e.clientX - _dragState.startX));
  _dragState.lastWidth = next;
}

function onPointerUp(e: PointerEvent) {
  if (!_dragState) return;
  const { resizer } = _dragState;
  if (
    typeof resizer.hasPointerCapture === 'function' &&
    typeof resizer.releasePointerCapture === 'function' &&
    resizer.hasPointerCapture(e.pointerId)
  ) {
    resizer.releasePointerCapture(e.pointerId);
  }
  document.removeEventListener('pointermove', onPointerMove);
  document.removeEventListener('pointerup', onPointerUp);
  document.body.classList.remove('sidebar-resizing');
  persistSidebarWidth(_dragState.lastWidth);
  _dragState = null;
}

export function attachNotesSidebarResize(aside: HTMLElement | null = document.getElementById('sidebar')) {
  detachNotesSidebarResize();
  if (!aside) return () => {};

  let resizer = aside.querySelector('#sidebar-resizer') as HTMLElement | null;
  if (!resizer) {
    resizer = document.createElement('div');
    resizer.id = 'sidebar-resizer';
    resizer.className = 'sidebar-resizer';
    resizer.setAttribute('role', 'separator');
    resizer.setAttribute('aria-orientation', 'vertical');
    resizer.setAttribute('aria-label', 'Resize sidebar');
    resizer.tabIndex = 0;
    aside.appendChild(resizer);
  }

  const saved = parseInt(globalThis.localStorage?.getItem?.(STORAGE_KEY) ?? '', 10);
  setSidebarWidth(aside, Number.isFinite(saved) ? saved : DEFAULT_WIDTH);

  const onPointerDown = (e: PointerEvent) => {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
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

export function detachNotesSidebarResize() {
  _cleanup?.();
  _cleanup = null;
}

/** Leftover / tests: attach to `#sidebar`. */
export function initSidebarResize() {
  attachNotesSidebarResize();
}

export { setSidebarWidth, STORAGE_KEY, DEFAULT_WIDTH, MIN_WIDTH, MAX_WIDTH };
