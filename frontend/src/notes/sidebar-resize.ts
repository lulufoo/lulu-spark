// @ts-nocheck — DOM wiring stays unchecked like checkJs:false.
const STORAGE_KEY = 'cta_sidebar_width';
const DEFAULT_WIDTH = 221;
const MIN_WIDTH = 180;
const MAX_WIDTH = 400;

let _resizeInitialized = false;
let _dragState = null;

function setSidebarWidth(aside, px) {
  const w = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, px));
  aside.style.setProperty('--sidebar-width', `${w}px`);
  aside.style.width = `${w}px`;
  aside.style.flexBasis = `${w}px`;
  return w;
}

function persistSidebarWidth(px) {
  localStorage.setItem(STORAGE_KEY, String(px));
}

function onPointerMove(e) {
  if (!_dragState) return;
  const next = setSidebarWidth(_dragState.aside, _dragState.startWidth + (e.clientX - _dragState.startX));
  _dragState.lastWidth = next;
}

function onPointerUp(e) {
  if (!_dragState) return;
  const resizer = _dragState.resizer;
  if (resizer.hasPointerCapture(e.pointerId)) {
    resizer.releasePointerCapture(e.pointerId);
  }
  document.removeEventListener('pointermove', onPointerMove);
  document.removeEventListener('pointerup', onPointerUp);
  document.body.classList.remove('sidebar-resizing');
  persistSidebarWidth(_dragState.lastWidth);
  _dragState = null;
}

function onPointerDown(e) {
  const aside = document.getElementById('sidebar');
  const resizer = document.getElementById('sidebar-resizer');
  if (!aside || !resizer || e.button !== 0) return;
  e.preventDefault();
  const startWidth = aside.getBoundingClientRect().width;
  _dragState = { aside, resizer, startX: e.clientX, startWidth, lastWidth: startWidth };
  resizer.setPointerCapture(e.pointerId);
  document.body.classList.add('sidebar-resizing');
  document.addEventListener('pointermove', onPointerMove);
  document.addEventListener('pointerup', onPointerUp);
}

export function initSidebarResize() {
  if (_resizeInitialized) return;
  const aside = document.getElementById('sidebar');
  const resizer = document.getElementById('sidebar-resizer');
  if (!aside || !resizer) return;
  _resizeInitialized = true;

  const saved = parseInt(localStorage.getItem(STORAGE_KEY), 10);
  setSidebarWidth(aside, Number.isFinite(saved) ? saved : DEFAULT_WIDTH);

  resizer.addEventListener('pointerdown', onPointerDown);
}
