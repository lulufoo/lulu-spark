import { notifyState, state } from '../host/state.ts';
import type { AppRoute } from './state/types.ts';
import { applySearchNavChrome } from './ui/nav-chrome.ts';
import { initSparkSearch } from '../notes/ui/search.tsx';
import { initKnowledgeSearch } from '../knowledge/ui/search.tsx';
import { clearHeaderSyncKnowledgeContext } from './commands/header-sync.ts';
import { openReadLaterDialog } from '../read-later/commands/dialog.ts';
import { openDoc } from '../notes/viewer.ts';
import { selectDate } from '../notes/commands/sidebar.ts';
import { getHomeEntryShell, setHomeEntryShell } from '../home-entry-shell/access.ts';
import { lastNotifyTrace, logNotifyHop, parseTraceId } from '../router/notify-trace.ts';

export { getHomeEntryShell, setHomeEntryShell };

function updateNavChrome(routeName: string) {
  const onHome = routeName === 'home';
  const homeTitle = document.getElementById('btn-nav-home-title');
  if (homeTitle) {
    homeTitle.hidden = false;
    homeTitle.classList.toggle('is-back', !onHome);
    homeTitle.setAttribute('aria-label', onHome ? 'Lulu Spark' : 'Home');
  }
  applySearchNavChrome(routeName);
}

export function wrapRouteMount(routeName: string, mountFn: (route: AppRoute) => unknown) {
  return (route: AppRoute) => {
    // Leave-host: force shell back to A so overlay never crosses pages.
    // @ts-expect-error leave-route source scan requires forceRecoverA(
    getHomeEntryShell()?.forceRecoverA('leave-route');
    updateNavChrome(routeName);
    if (routeName === 'spark') initSparkSearch();
    if (routeName === 'knowledge-doc') initKnowledgeSearch();
    return mountFn(route);
  };
}

export function mountHomeRoute() {
  clearHeaderSyncKnowledgeContext();
}

export function mountKnowledgeDocRoute() {}

export function mountReadLaterRoute() {
  mountHomeRoute();
  openReadLaterDialog();
}

export function mountSpark(route?: AppRoute) {
  clearHeaderSyncKnowledgeContext();

  const params = route?.params || {};
  const notePath = params.note || '';
  const date = params.date || '';
  const layer = params.layer || 'raw';
  const creating = !!state.viewer?.createSession;
  const hash = typeof window !== 'undefined' ? window.location.hash : '';
  logNotifyHop('route.spark', parseTraceId(params.trace) ?? lastNotifyTrace(), {
    outcome: notePath ? 'ok' : 'no_note',
    date,
    note: notePath,
    layer,
    hash,
  });

  if (notePath) {
    const allEntries = Object.values(state.index?.data || {});
    let entry = allEntries.find((e) => e.common_path === notePath);
    if (!entry) entry = allEntries.find((e) => e.translations?.zh === notePath);
    if (entry) {
      state.viewer.outletMode = 'open';
      state.viewer.outletMessage = '';
      notifyState();
      void openDoc(entry, layer);
    } else {
      // note-outlet-message is painted by NotesPage from this snapshot.
      state.viewer.outletMode = 'safe-empty';
      state.viewer.outletMessage = `Note not found: ${notePath}`;
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
