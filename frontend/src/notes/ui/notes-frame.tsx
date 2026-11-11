import { useLayoutEffect, useRef, useState, useSyncExternalStore, type ReactNode } from 'react';
import { restoreSidebarCollapsed, toggleSidebarCollapsed } from '../../app-shell/commands/sidebar-collapsed.ts';
import { sidebarCollapsedStore } from '../../app-shell/state/sidebar-collapsed.ts';
import { SidebarLayout } from '../../app-shell/ui/sidebar-layout.tsx';
import { SidebarToggle } from '../../app-shell/ui/sidebar-toggle.tsx';
import { PageBackHome } from '../../shared/home-mark.tsx';
import { WindowDragStrip } from '../../shared/window-drag-strip.tsx';
import { readNotesSidebarWidth, notesSidebarWidthStore } from '../state/sidebar-width.ts';
import { NotesHeaderSlot } from './header-slot.tsx';
import { NotesSidebar } from './sidebar.tsx';

/** List and article shell. The header slot is filled by the page; the body stays the child. */
export function NotesFrame({
  activeDate = '',
  children,
}: {
  activeDate?: string;
  children: ReactNode;
}) {
  const collapsed = useSyncExternalStore(
    sidebarCollapsedStore.subscribe,
    sidebarCollapsedStore.getSnapshot,
  );
  const width = useSyncExternalStore(
    notesSidebarWidthStore.subscribe,
    notesSidebarWidthStore.getSnapshot,
  );
  const headerRef = useRef<HTMLDivElement>(null);
  const [headerEl, setHeaderEl] = useState<HTMLElement | null>(null);

  useLayoutEffect(() => {
    notesSidebarWidthStore.set(readNotesSidebarWidth());
    restoreSidebarCollapsed();
    setHeaderEl(headerRef.current);
  }, []);

  return (
    <NotesHeaderSlot.Provider value={headerEl}>
      <SidebarLayout
        collapsed={collapsed}
        width={`${width}px`}
        navId="notes-sidebar-nav"
        tool={(
          <WindowDragStrip>
            <span className="sidebar-tool-pair">
              <PageBackHome />
              <SidebarToggle
                collapsed={collapsed}
                controlsId="notes-sidebar-nav"
                onToggle={toggleSidebarCollapsed}
              />
            </span>
          </WindowDragStrip>
        )}
        header={<div ref={headerRef} className="notes-header-slot" />}
        nav={(
          <aside id="sidebar">
            <NotesSidebar collapsed={collapsed} />
          </aside>
        )}
      >
        <main id="main" data-active-date={activeDate}>
          {children}
          <div id="feed-view" hidden />
        </main>
      </SidebarLayout>
    </NotesHeaderSlot.Provider>
  );
}
