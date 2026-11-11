import {
  readSidebarLayoutStartWidth,
  sidebarResizeHost,
} from '../../app-shell/ui/sidebar-resize-host.ts';
import {
  NOTES_SIDEBAR_DEFAULT_WIDTH,
  NOTES_SIDEBAR_MAX_WIDTH,
  NOTES_SIDEBAR_MIN_WIDTH,
  NOTES_SIDEBAR_WIDTH_KEY,
  notesSidebarWidthStore,
  readNotesSidebarWidth,
  rememberNotesSidebarWidth,
} from '../state/sidebar-width.ts';

const STORAGE_KEY = NOTES_SIDEBAR_WIDTH_KEY;
const DEFAULT_WIDTH = NOTES_SIDEBAR_DEFAULT_WIDTH;
const MIN_WIDTH = NOTES_SIDEBAR_MIN_WIDTH;
const MAX_WIDTH = NOTES_SIDEBAR_MAX_WIDTH;

type DragState = {
  aside: HTMLElement;
  resizer: HTMLElement;
  startX: number;
  startWidth: number;
  lastWidth: number;
};

let _cleanup: (() => void) | null = null;
let _dragState: DragState | null = null;

function paintSidebarWidth(aside: HTMLElement, px: number) {
  const w = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, px));
  aside.style.setProperty('--sidebar-width', `${w}px`);
  aside.style.width = `${w}px`;
  aside.style.flexBasis = `${w}px`;
  const root = aside.closest('.sidebar-layout');
  if (root instanceof HTMLElement) root.style.setProperty('--sidebar-layout-width', `${w}px`);
  return w;
}

function setSidebarWidth(aside: HTMLElement, px: number) {
  const w = paintSidebarWidth(aside, px);
  notesSidebarWidthStore.set(w);
  return w;
}

function persistSidebarWidth(px: number) {
  rememberNotesSidebarWidth(px);
}

function onPointerMove(e: PointerEvent) {
  if (!_dragState) return;
  const next = paintSidebarWidth(_dragState.aside, _dragState.startWidth + (e.clientX - _dragState.startX));
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

  const host = sidebarResizeHost(aside);
  let resizer = host.querySelector('#sidebar-resizer') as HTMLElement | null;
  if (!resizer) {
    resizer = document.createElement('div');
    resizer.id = 'sidebar-resizer';
    resizer.className = 'sidebar-resizer';
    resizer.setAttribute('role', 'separator');
    resizer.setAttribute('aria-orientation', 'vertical');
    resizer.setAttribute('aria-label', 'Resize sidebar');
    resizer.tabIndex = 0;
    host.appendChild(resizer);
  }

  setSidebarWidth(aside, readNotesSidebarWidth());

  const onPointerDown = (e: PointerEvent) => {
    if (e.button !== 0) return;
    if (aside.closest('.sidebar-layout')?.getAttribute('data-collapsed') === 'true') return;
    e.preventDefault();
    e.stopPropagation();
    const startWidth = readSidebarLayoutStartWidth(aside);
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
