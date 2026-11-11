/** The layout root owns the column width; fall back to the aside when there is no layout. */
export function sidebarResizeHost(aside: HTMLElement): HTMLElement {
  const root = aside.closest('.sidebar-layout');
  return root instanceof HTMLElement ? root : aside;
}

/** Drag start width: layout token, then aside.style.width, then the box. */
export function readSidebarLayoutStartWidth(aside: HTMLElement): number {
  const host = sidebarResizeHost(aside);
  return Number.parseFloat(
    host.style.getPropertyValue('--sidebar-layout-width') || aside.style.width || '0',
  ) || aside.getBoundingClientRect().width;
}
