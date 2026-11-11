import { useLayoutEffect, useRef, type ReactNode } from 'react';
import { SidebarToggle } from './sidebar-toggle.tsx';

type SidebarLayoutProps = {
  collapsed: boolean;
  /** Saved sidebar width, such as `248px`. Collapse does not change it. */
  width: string;
  /** Stable id of the nav slot. The toggle's aria-controls points here. */
  navId: string;
  onToggle: () => void;
  tool: ReactNode;
  header: ReactNode;
  /** Lower navigation, including the drag handle. Hidden as one piece. */
  nav: ReactNode;
  children: ReactNode;
  resizer?: ReactNode;
};

function hasFocusInside(nav: HTMLElement | null): boolean {
  const active = document.activeElement;
  return Boolean(nav && active instanceof HTMLElement && nav.contains(active));
}

function toggleHasFocus(): boolean {
  const active = document.activeElement;
  return active instanceof HTMLElement && Boolean(active.closest('[data-sidebar-toggle]'));
}

/** Two-row grid. Collapsed, the tool cell hides and the show button leads the header. */
export function SidebarLayout({
  collapsed,
  width,
  navId,
  onToggle,
  tool,
  header,
  nav,
  children,
  resizer,
}: SidebarLayoutProps) {
  const rootRef = useRef<HTMLDivElement>(null);
  const navRef = useRef<HTMLDivElement>(null);
  const wasCollapsedRef = useRef(collapsed);
  const moveFocusRef = useRef(false);

  if (
    collapsed !== wasCollapsedRef.current
    && (toggleHasFocus() || (collapsed && hasFocusInside(navRef.current)))
  ) {
    moveFocusRef.current = true;
  }
  wasCollapsedRef.current = collapsed;

  useLayoutEffect(() => {
    if (!moveFocusRef.current) return;
    moveFocusRef.current = false;
    const slot = collapsed ? '.sidebar-header-restore' : '.sidebar-layout-tool';
    rootRef.current?.querySelector<HTMLElement>(`${slot} [data-sidebar-toggle]`)?.focus();
  }, [collapsed]);

  return (
    <div
      ref={rootRef}
      className="sidebar-layout"
      data-collapsed={collapsed ? 'true' : 'false'}
      style={{ ['--sidebar-layout-width' as string]: width }}
    >
      <div className="sidebar-layout-tool" hidden={collapsed}>{tool}</div>
      <div className="sidebar-layout-header">
        <div className="sidebar-header-restore" hidden={!collapsed}>
          <SidebarToggle collapsed={collapsed} controlsId={navId} onToggle={onToggle} />
        </div>
        {header}
      </div>
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
      {resizer}
    </div>
  );
}
