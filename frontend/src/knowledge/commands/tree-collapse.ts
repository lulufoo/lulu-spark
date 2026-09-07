import {
  persistKnowledgeTreeCollapsed,
  readKnowledgeTreeCollapsed,
} from '../state/tree-collapse.ts';

let _cleanup: (() => void) | null = null;

export function layoutForTreeToggle(from: HTMLElement | null): HTMLElement | null {
  return from?.closest('.knowledge-doc-layout') ?? null;
}

export function paintKnowledgeTreeToggle(btn: HTMLElement | null, collapsed: boolean) {
  if (!btn) return;
  const title = collapsed ? 'Expand directory' : 'Collapse directory';
  btn.classList.toggle('is-collapsed', collapsed);
  btn.title = title;
  btn.setAttribute('aria-label', title);
  btn.setAttribute('aria-expanded', collapsed ? 'false' : 'true');
}

export function applyKnowledgeTreeCollapsed(layout: HTMLElement | null, collapsed: boolean) {
  if (!layout) return;
  layout.classList.toggle('is-tree-collapsed', collapsed);
  persistKnowledgeTreeCollapsed(collapsed);
  paintKnowledgeTreeToggle(layout.querySelector('.kb-btn-tree-toggle'), collapsed);
}

export function restoreKnowledgeTreeCollapsed(layout: HTMLElement | null) {
  applyKnowledgeTreeCollapsed(layout, readKnowledgeTreeCollapsed());
}

export function toggleKnowledgeTreeCollapsed(from: HTMLElement | null) {
  const layout = layoutForTreeToggle(from);
  const next = layout
    ? !layout.classList.contains('is-tree-collapsed')
    : !readKnowledgeTreeCollapsed();
  if (layout) {
    applyKnowledgeTreeCollapsed(layout, next);
    return;
  }
  persistKnowledgeTreeCollapsed(next);
  const btn = from?.closest('.kb-btn-tree-toggle') ?? from;
  paintKnowledgeTreeToggle(btn, next);
}

export function syncKnowledgeTreeToggle(btn: HTMLElement | null) {
  const layout = layoutForTreeToggle(btn);
  const collapsed = layout
    ? layout.classList.contains('is-tree-collapsed')
    : readKnowledgeTreeCollapsed();
  paintKnowledgeTreeToggle(btn, collapsed);
}

function isTreeToggleClick(event: Event, layout: HTMLElement) {
  const target = event.target;
  if (!(target instanceof Element)) return false;
  const btn = target.closest('.kb-btn-tree-toggle');
  return Boolean(btn && layout.contains(btn));
}

export function attachKnowledgeTreeToggle(layout: HTMLElement | null) {
  detachKnowledgeTreeToggle();
  if (!layout) return () => {};
  restoreKnowledgeTreeCollapsed(layout);
  const onClick = (event: Event) => {
    if (!isTreeToggleClick(event, layout)) return;
    toggleKnowledgeTreeCollapsed(event.target as HTMLElement);
  };
  layout.addEventListener('click', onClick);
  _cleanup = () => {
    layout.removeEventListener('click', onClick);
  };
  return _cleanup;
}

export function detachKnowledgeTreeToggle() {
  _cleanup?.();
  _cleanup = null;
}
