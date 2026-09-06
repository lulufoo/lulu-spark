/**
 * Position a fixed list menu near a trigger; expand to natural height when space allows.
 * @returns true when menu is clipped and caller should scroll selected item into view
 */
export function positionFloatingListMenu(
  menu: HTMLElement,
  rect: DOMRect,
  viewportHeight = window.innerHeight,
  maxHeightCap?: number,
): boolean {
  const gap = 4;
  const margin = 8;
  const cap = maxHeightCap != null && maxHeightCap > 0 ? maxHeightCap : Number.POSITIVE_INFINITY;

  menu.style.left = `${rect.left}px`;
  menu.style.width = `${rect.width}px`;
  menu.style.maxHeight = 'none';
  menu.style.visibility = 'hidden';
  const naturalHeight = menu.scrollHeight;
  menu.style.visibility = '';

  const spaceBelow = viewportHeight - rect.bottom - margin;
  const spaceAbove = rect.top - margin;
  const clippedMaxBelow = Math.min(Math.floor(spaceBelow * 0.8), cap);
  const clippedMaxAbove = Math.min(Math.floor(spaceAbove * 0.8), cap);
  let top: number;
  let maxHeight: number | null = null;

  if (naturalHeight <= spaceBelow && naturalHeight <= cap) {
    top = rect.bottom + gap;
  } else if (naturalHeight <= spaceAbove && naturalHeight <= cap) {
    top = rect.top - naturalHeight - gap;
  } else if (spaceBelow >= spaceAbove) {
    top = rect.bottom + gap;
    maxHeight = clippedMaxBelow;
  } else {
    maxHeight = clippedMaxAbove;
    top = Math.max(margin, rect.top - maxHeight - gap);
  }

  menu.style.top = `${top}px`;
  if (maxHeight != null) {
    menu.style.maxHeight = `${maxHeight}px`;
  } else {
    menu.style.maxHeight = '';
  }

  return naturalHeight > (maxHeight ?? naturalHeight);
}
