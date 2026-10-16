import { notifyState, state } from '../host/state.ts';
import type { AppRoute } from './state/types.ts';
import { applySearchNavChrome } from './ui/nav-chrome.ts';
import { initSparkSearch } from '../notes/ui/search.tsx';
import { openReadLaterDialog } from '../read-later/commands/dialog.ts';
import { openDoc } from '../notes/viewer.ts';
import { selectDate } from '../notes/commands/sidebar.ts';
import { getHomeEntryShell, setHomeEntryShell } from '../home-entry-shell/access.ts';
import { lastNotifyTrace, logNotifyHop, parseTraceId } from '../router/notify-trace.ts';

export { getHomeEntryShell, setHomeEntryShell };

function updateNavChrome(routeName: string) {
  applySearchNavChrome(routeName);
}

export function wrapRouteMount(routeName: string, mountFn: (route: AppRoute) => unknown) {
  return (route: AppRoute) => {
    // Leave-host: force shell back to A so overlay never crosses pages.
    // @ts-expect-error leave-route source scan requires forceRecoverA(
    getHomeEntryShell()?.forceRecoverA('leave-route');
    updateNavChrome(routeName);
    if (routeName === 'spark') initSparkSearch();
    return mountFn(route);
  };
}

export function mountHomeRoute() {}

export function mountReadLaterRoute() {
  mountHomeRoute();
  openReadLaterDialog();
}

export function mountSpark(route?: AppRoute) {
  const params = route?.params || {};
  const noteId = params.note || '';
  const date = params.date || '';
  const layer = params.layer || 'raw';
  const creating = !!state.viewer?.createSession;
  const hash = typeof window !== 'undefined' ? window.location.hash : '';
  logNotifyHop('route.spark', parseTraceId(params.trace) ?? lastNotifyTrace(), {
    outcome: noteId ? 'ok' : 'no_note',
    date,
    note: noteId,
    layer,
    hash,
  });

  if (noteId) {
    const data = state.index?.data || {};
    const entry = data[noteId] ?? Object.values(data).find((e) => e._id === noteId || e.id === noteId);
    if (entry) {
      state.viewer.outletMode = 'open';
      state.viewer.outletMessage = '';
      notifyState();
      void openDoc(entry, layer);
    } else {
      state.viewer.outletMode = 'safe-empty';
      state.viewer.outletMessage = `Note not found: ${noteId}`;
      notifyState();
    }
    return;
  }

  if (creating) {
    state.viewer.outletMode = 'create';
    state.viewer.outletMessage = '';
    notifyState();
    return;
  }

  state.viewer.outletMode = '';
  state.viewer.outletMessage = '';
  notifyState();
  if (date && typeof selectDate === 'function') selectDate(date);
}
