// @ts-nocheck — DOM wiring stays unchecked like checkJs:false.
import { notifyState, state } from '../host/state.ts';
import { applySearchNavChrome } from './ui/nav-chrome.ts';
import { initWorkbenchSearch } from '../notes/ui/search.tsx';
import { initCorpusSearch } from '../corpus/ui/search.tsx';
import { clearHeaderSyncCorpusContext } from './commands/header-sync.ts';
import { openReadLaterDialog } from '../read-later/commands/dialog.ts';
import { openDoc } from '../notes/viewer.ts';
import { selectDate } from '../notes/commands/sidebar.ts';
import { getHomeEntryShell, setHomeEntryShell } from '../home-entry-shell/access.ts';

export { getHomeEntryShell, setHomeEntryShell };


function updateNavChrome(routeName) {
  const onHome = routeName === 'home';
  const homeTitle = document.getElementById('btn-nav-home-title');
  const homeNav = document.getElementById('btn-nav-home');
  if (homeTitle) homeTitle.hidden = !onHome;
  if (homeNav) homeNav.hidden = onHome;
  applySearchNavChrome(routeName);
}

export function wrapRouteMount(routeName, mountFn) {
  return (route) => {
    // Leave-host: force shell back to A so overlay never crosses pages.
    getHomeEntryShell()?.forceRecoverA('leave-route');
    updateNavChrome(routeName);
    if (routeName === 'workbench') initWorkbenchSearch();
    if (routeName === 'corpus-doc') initCorpusSearch();
    return mountFn(route);
  };
}

function hideHomeView() {
  const homeView = document.getElementById('home-view');
  if (homeView) homeView.style.display = 'none';
}

function hideCorpusDocView() {
  const docView = document.getElementById('corpus-doc-view');
  if (docView) docView.style.display = 'none';
  const layout = document.querySelector('.layout');
  if (layout) layout.style.display = '';
}

function hideReadLaterView() {
  const readLaterView = document.getElementById('read-later-view');
  if (readLaterView) readLaterView.style.display = 'none';
}

function hideTodoTasksView() {
  const todoTasksView = document.getElementById('todo-tasks-view');
  if (todoTasksView) todoTasksView.style.display = 'none';
}

export function mountHomeRoute() {
  clearHeaderSyncCorpusContext();
  hideCorpusDocView();
  hideReadLaterView();
  hideTodoTasksView();

  const feedView = document.getElementById('feed-view');
  if (feedView) feedView.style.display = 'none';

  const layout = document.querySelector('.layout');
  if (layout) layout.style.display = 'none';

  const homeView = document.getElementById('home-view');
  if (!homeView) return;
  homeView.style.display = '';
}

export function mountCorpusDocRoute() {
  hideHomeView();
  hideReadLaterView();
  hideTodoTasksView();

  const layout = document.querySelector('.layout');
  if (layout) layout.style.display = 'none';

  const docView = document.getElementById('corpus-doc-view');
  if (!docView) return;
  docView.style.display = '';
}

export function mountReadLaterRoute() {
  mountHomeRoute();
  openReadLaterDialog();
}

export function mountTodoTasksRoute() {
  clearHeaderSyncCorpusContext();
  hideHomeView();
  hideCorpusDocView();
  hideReadLaterView();

  const feedView = document.getElementById('feed-view');
  if (feedView) feedView.style.display = 'none';

  const layout = document.querySelector('.layout');
  if (layout) layout.style.display = 'none';

  const todoTasksView = document.getElementById('todo-tasks-view');
  if (!todoTasksView) return;
  todoTasksView.style.display = '';
}

export function mountWorkbench(route) {
  clearHeaderSyncCorpusContext();
  hideHomeView();
  hideCorpusDocView();
  hideReadLaterView();
  hideTodoTasksView();

  const feedView = document.getElementById('feed-view');
  if (feedView) feedView.style.display = 'none';

  const params = route?.params || {};
  const notePath = params.note || '';
  const date = params.date || '';
  const layer = params.layer || 'raw';
  const creating = !!state.viewer?.createSession;

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
