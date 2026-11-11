import {
  recallSidebarCollapsed,
  rememberSidebarCollapsed,
  sidebarCollapsedStore,
} from '../state/sidebar-collapsed.ts';

/** Load the shared preference into the session. */
export function restoreSidebarCollapsed(): boolean {
  const collapsed = recallSidebarCollapsed();
  sidebarCollapsedStore.set(collapsed);
  return collapsed;
}

/** Flip the shared flag and remember it. Returns the value now showing. */
export function toggleSidebarCollapsed(): boolean {
  const next = !sidebarCollapsedStore.getSnapshot();
  rememberSidebarCollapsed(next);
  return next;
}
