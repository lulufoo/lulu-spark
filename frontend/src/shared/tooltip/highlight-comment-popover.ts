type Point = { x: number; y: number };

export function highlightCommentFragment(anchor: HTMLElement, point?: Point): number {
  if (!point) return 0;
  // A wrapped highlight can have a separate rectangle on each line.
  const index = Array.from(anchor.getClientRects()).findIndex((rect) => (
    point.x >= rect.left && point.x <= rect.right && point.y >= rect.top && point.y <= rect.bottom
  ));
  return Math.max(0, index);
}

export function placeHighlightCommentPopover(panel: HTMLElement, anchor: HTMLElement, fragment: number): void {
  const rect = anchor.getClientRects()[fragment] ?? anchor.getBoundingClientRect();
  const margin = 8;
  const gap = 12;
  const below = rect.bottom + gap;
  const availableHeight = Math.max(0, rect.top - gap - margin, window.innerHeight - margin - below);
  panel.style.setProperty('--highlight-comment-available-height', `${availableHeight}px`);
  const { width, height } = panel.getBoundingClientRect();
  const anchorX = rect.right - Math.min(8, rect.width / 2);
  const maxLeft = Math.max(margin, window.innerWidth - width - margin);
  const left = Math.min(maxLeft, Math.max(margin, anchorX - width * 0.74));
  const above = rect.top - height - gap;
  const maxTop = Math.max(margin, window.innerHeight - height - margin);
  const flip = below > maxTop && (above >= margin || rect.top > window.innerHeight - rect.bottom);
  const top = Math.min(maxTop, Math.max(margin, flip ? above : below));
  const arrowMargin = Math.min(14, width / 2);
  const arrow = Math.min(width - arrowMargin, Math.max(arrowMargin, anchorX - left));

  panel.style.left = `${left}px`;
  panel.style.top = `${top}px`;
  panel.style.setProperty('--highlight-comment-arrow', `${arrow}px`);
  panel.classList.toggle('is-above', flip);
}
