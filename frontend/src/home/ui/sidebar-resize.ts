import {
  HOME_SIDEBAR_DEFAULT_WIDTH,
  HOME_SIDEBAR_MAX_WIDTH,
  HOME_SIDEBAR_MIN_WIDTH,
  HOME_SIDEBAR_WIDTH_KEY,
  homeSidebarWidthStore,
  readHomeSidebarWidth,
  rememberHomeSidebarWidth,
} from '../state/sidebar-width.ts';

const STORAGE_KEY = HOME_SIDEBAR_WIDTH_KEY;
const DEFAULT_WIDTH = HOME_SIDEBAR_DEFAULT_WIDTH;
const MIN_WIDTH = HOME_SIDEBAR_MIN_WIDTH;
const MAX_WIDTH = HOME_SIDEBAR_MAX_WIDTH;

type DragState = {
  aside: HTMLElement;
  resizer: HTMLElement;
  startX: number;
  startWidth: number;
  lastWidth: number;
};

let _cleanup: (() => void) | null = null;
let _dragState: DragState | null = null;

function paintHomeSidebarWidth(aside: HTMLElement, px: number) {
  const w = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, px));
  aside.style.setProperty('--home-chat-sidebar-width', `${w}px`);
  aside.style.width = `${w}px`;
  aside.style.flexBasis = `${w}px`;
  const root = aside.closest('.sidebar-layout');
  if (root instanceof HTMLElement) root.style.setProperty('--sidebar-layout-width', `${w}px`);
  return w;
}

function setHomeSidebarWidth(aside: HTMLElement, px: number) {
  const w = paintHomeSidebarWidth(aside, px);
  homeSidebarWidthStore.set(w);
  return w;
}

function persistHomeSidebarWidth(px: number) {
  rememberHomeSidebarWidth(px);
}

function onPointerMove(e: PointerEvent) {
  if (!_dragState) return;
  const next = paintHomeSidebarWidth(
    _dragState.aside,
    _dragState.startWidth + (e.clientX - _dragState.startX),
  );
  _dragState.lastWidth = next;
}

function onPointerUp(e: PointerEvent) {
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
  persistHomeSidebarWidth(_dragState.lastWidth);
  _dragState = null;
}

export function attachHomeSidebarResize(aside: HTMLElement) {
  detachHomeSidebarResize();
  if (!aside) return () => {};

  let resizer = aside.querySelector('.home-chat-sidebar-resizer') as HTMLElement | null;
  if (!resizer) {
    resizer = document.createElement('div');
    resizer.className = 'home-chat-sidebar-resizer sidebar-resizer';
    resizer.setAttribute('role', 'separator');
    resizer.setAttribute('aria-orientation', 'vertical');
    resizer.setAttribute('aria-label', 'Resize sidebar');
    resizer.tabIndex = 0;
    aside.appendChild(resizer);
  }

  setHomeSidebarWidth(aside, readHomeSidebarWidth());

  const onPointerDown = (e: PointerEvent) => {
    if (e.button !== 0) return;
    if (aside.closest('.sidebar-layout')?.getAttribute('data-collapsed') === 'true') return;
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

export function detachHomeSidebarResize() {
  _cleanup?.();
  _cleanup = null;
}

export { setHomeSidebarWidth, STORAGE_KEY, DEFAULT_WIDTH, MIN_WIDTH, MAX_WIDTH };
