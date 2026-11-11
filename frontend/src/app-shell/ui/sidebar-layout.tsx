import { useLayoutEffect, useRef, type ReactNode } from 'react';

type SidebarLayoutProps = {
  collapsed: boolean;
  /** Saved sidebar width, such as `248px`. Collapse does not change it. */
  width: string;
  /** Stable id of the nav slot. The toggle's aria-controls points here. */
  navId: string;
  tool: ReactNode;
  header: ReactNode;
  /** Lower navigation, including the drag handle. Hidden as one piece. */
  nav: ReactNode;
  children: ReactNode;
};

function hasFocusInside(nav: HTMLElement | null): boolean {
  const active = document.activeElement;
  return Boolean(nav && active instanceof HTMLElement && nav.contains(active));
}

/** Two-row grid. The caller passes the collapsed flag and the four slots. */
export function SidebarLayout({
  collapsed,
  width,
  navId,
  tool,
  header,
  nav,
  children,
}: SidebarLayoutProps) {
  const rootRef = useRef<HTMLDivElement>(null);
  const navRef = useRef<HTMLDivElement>(null);
  const wasCollapsedRef = useRef(collapsed);
  const restoreFocusRef = useRef(false);

  if (collapsed && !wasCollapsedRef.current && hasFocusInside(navRef.current)) {
    restoreFocusRef.current = true;
  }
  wasCollapsedRef.current = collapsed;

  useLayoutEffect(() => {
    if (!restoreFocusRef.current) return;
    restoreFocusRef.current = false;
    rootRef.current?.querySelector<HTMLElement>('[data-sidebar-toggle]')?.focus();
  }, [collapsed]);

  return (
    <div
      ref={rootRef}
      className="sidebar-layout"
      data-collapsed={collapsed ? 'true' : 'false'}
      style={{ ['--sidebar-layout-width' as string]: width }}
    >
      <div className="sidebar-layout-tool">{tool}</div>
      <div className="sidebar-layout-header">{header}</div>
      <div
        ref={navRef}
        id={navId}
        className="sidebar-layout-nav"
        hidden={collapsed}
        inert={collapsed}
      >
        {nav}
      </div>
      <div className="sidebar-layout-content">{children}</div>
    </div>
  );
}
