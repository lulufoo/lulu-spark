/**
 * Position a fixed list menu near a trigger; expand to natural height when space allows.
 * @param {HTMLElement} menu
 * @param {DOMRect} rect
 * @param {number} [viewportHeight]
 * @returns {boolean} true when menu is clipped and caller should scroll selected item into view
 */
export function positionFloatingListMenu(menu, rect, viewportHeight = window.innerHeight) {
  const gap = 4;
  const margin = 8;

  menu.style.left = `${rect.left}px`;
  menu.style.width = `${rect.width}px`;
  menu.style.maxHeight = 'none';
  menu.style.visibility = 'hidden';
  const naturalHeight = menu.scrollHeight;
  menu.style.visibility = '';

  const spaceBelow = viewportHeight - rect.bottom - margin;
  const spaceAbove = rect.top - margin;
  const clippedMaxBelow = Math.floor(spaceBelow * 0.8);
  const clippedMaxAbove = Math.floor(spaceAbove * 0.8);
  let top;
  let maxHeight = null;

  if (naturalHeight <= spaceBelow) {
    top = rect.bottom + gap;
  } else if (naturalHeight <= spaceAbove) {
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
