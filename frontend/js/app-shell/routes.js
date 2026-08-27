import { state } from '../host/state.js';
import { applySearchNavChrome } from './nav-chrome.js';
import { initWorkbenchSearch } from '../notes/search.js';
import { initCorpusSearch } from '../corpus/corpus-search.js';
import { mountCorpusDocList } from '../corpus/corpus-doc-list.js';
import { mountHomeHub } from '../home-entry-shell/hub.js';
import { mountTodoTaskSplit } from '../todo-task/index.js';
import { clearHeaderSyncCorpusContext } from './header-sync.js';
import { navigate } from '../router/index.js';
import { openReadLaterDialog } from '../read-later/dialog.js';
import { openDoc } from '../notes/viewer.js';
import { selectDate } from '../notes/sidebar.js';

const feedView = document.getElementById('feed-view');

let unmountCorpusDocList = null;
let corpusDocListRepo = '';
let unmountHomeHub = null;
let unmountTodoTaskSplit = null;
/** @type {{ forceRecoverA?: (reason: string) => void } | null} */
let homeEntryShell = null;

export function setHomeEntryShell(shell) {
  homeEntryShell = shell;
}

export function getHomeEntryShell() {
  return homeEntryShell;
}

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
    homeEntryShell?.forceRecoverA('leave-route');
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
  unmountCorpusDocList?.();
  unmountCorpusDocList = null;
  corpusDocListRepo = '';
  hideCorpusDocView();
  hideReadLaterView();
  hideTodoTasksView();

  if (feedView) feedView.style.display = 'none';

  const layout = document.querySelector('.layout');
  if (layout) layout.style.display = 'none';

  const homeView = document.getElementById('home-view');
  if (!homeView) return;
  homeView.style.display = '';

  unmountTodoTaskSplit?.();
  unmountTodoTaskSplit = null;
  unmountHomeHub?.();
  unmountHomeHub = mountHomeHub(homeView, { navigate, openReadLater: openReadLaterDialog });
}

export function mountCorpusDocRoute(route) {
  unmountHomeHub?.();
  unmountHomeHub = null;
  unmountTodoTaskSplit?.();
  unmountTodoTaskSplit = null;
  hideHomeView();
  hideReadLaterView();
  hideTodoTasksView();

  const layout = document.querySelector('.layout');
  if (layout) layout.style.display = 'none';

  const docView = document.getElementById('corpus-doc-view');
  if (!docView) return;
  docView.style.display = '';

  const repo = route?.params?.repo || '';
  const initialPath = route?.params?.path || '';

  if (unmountCorpusDocList && corpusDocListRepo === repo) {
    // Same repo: update path in-place. Do not remount — remount resets expanded tree state.
    void unmountCorpusDocList.navigateToPath?.(initialPath);
    return;
  }

  unmountCorpusDocList?.();
  unmountCorpusDocList = null;
  corpusDocListRepo = '';

  unmountCorpusDocList = mountCorpusDocList(docView, { repo, navigate, initialPath });
  corpusDocListRepo = unmountCorpusDocList.repo ?? repo;
}

export function mountReadLaterRoute() {
  mountHomeRoute();
  openReadLaterDialog();
}

export function mountTodoTasksRoute(route) {
  clearHeaderSyncCorpusContext();
  unmountHomeHub?.();
  unmountHomeHub = null;
  unmountCorpusDocList?.();
  unmountCorpusDocList = null;
  corpusDocListRepo = '';
  hideHomeView();
  hideCorpusDocView();
  hideReadLaterView();

  if (feedView) feedView.style.display = 'none';

  const layout = document.querySelector('.layout');
  if (layout) layout.style.display = 'none';

  const todoTasksView = document.getElementById('todo-tasks-view');
  if (!todoTasksView) return;
  todoTasksView.style.display = '';

  const masterId = route?.params?.master ?? '';
  const subId = route?.params?.sub ?? '';
  // Same page: update selection in-place. Remount would reset pane scroll positions.
  if (typeof unmountTodoTaskSplit?.applyRoute === 'function') {
    unmountTodoTaskSplit.applyRoute({ masterId, subId });
    return;
  }

  unmountTodoTaskSplit?.();
  const mounted = mountTodoTaskSplit(todoTasksView, {
    masterId,
    subId,
    navigate,
  });
  unmountTodoTaskSplit = mounted.unmount;
}

export function mountWorkbench(route) {
  clearHeaderSyncCorpusContext();
  unmountHomeHub?.();
  unmountHomeHub = null;
  unmountTodoTaskSplit?.();
  unmountTodoTaskSplit = null;
  hideHomeView();
  unmountCorpusDocList?.();
  unmountCorpusDocList = null;
  corpusDocListRepo = '';
  hideCorpusDocView();
  hideReadLaterView();
  hideTodoTasksView();

  if (feedView) feedView.style.display = 'none';
  // Restore archive elements to their natural display state.
  // date-heading is list-only — note/create branches hide it (avoid a second chrome row).
  const status = document.getElementById('status');
  const dateHeading = document.getElementById('date-heading');
  const docList = document.getElementById('doc-list');
  if (state.ui.activeDate) {
    if (status) status.style.display = 'none';
  } else {
    if (status) status.style.display = '';
  }

  const params = route?.params || {};
  const notePath = params.note || '';
  const date = params.date || '';
  const layer = params.layer || 'raw';
  const creating = !!state.viewer?.createSession;

  const ensureOutlet = () => {
    let outlet = document.getElementById('note-outlet');
    if (outlet) return outlet;
    outlet = document.createElement('div');
    outlet.id = 'note-outlet';
    outlet.hidden = true;
    const msg = document.createElement('div');
    msg.id = 'note-outlet-message';
    msg.className = 'note-outlet-message';
    msg.hidden = true;
    outlet.appendChild(msg);
    document.getElementById('main')?.appendChild(outlet);
    return outlet;
  };

  const activateOutlet = (mode, { note, layer: lyr, message = '' } = {}) => {
    const outlet = ensureOutlet();
    if (dateHeading) dateHeading.style.display = 'none';
    if (docList) docList.style.display = 'none';
    outlet.hidden = false;
    outlet.dataset.wbMode = mode;
    if (note) outlet.dataset.note = note;
    else delete outlet.dataset.note;
    if (lyr) outlet.dataset.layer = lyr;
    else delete outlet.dataset.layer;

    // Preserve #md-panel chrome — never wipe via textContent.
    let msgEl = document.getElementById('note-outlet-message');
    if (!msgEl) {
      msgEl = document.createElement('div');
      msgEl.id = 'note-outlet-message';
      msgEl.className = 'note-outlet-message';
      outlet.prepend(msgEl);
    }
    const panel = document.getElementById('md-panel');
    if (message) {
      msgEl.hidden = false;
      msgEl.textContent = message;
      if (panel) panel.hidden = true;
    } else {
      msgEl.hidden = true;
      msgEl.textContent = '';
      if (panel) panel.hidden = false;
    }
  };

  if (notePath) {
    const allEntries = Object.values(state.index?.data || {});
    let entry = allEntries.find((e) => e.common_path === notePath);
    if (!entry) entry = allEntries.find((e) => e.translations?.zh === notePath);
    if (entry) {
      activateOutlet('open', { note: entry.common_path, layer });
      void openDoc(entry, layer);
    } else {
      activateOutlet('safe-empty', {
        note: notePath,
        message: `Note not found: ${notePath}`,
      });
    }
    return;
  }

  if (creating) {
    activateOutlet('create');
    return;
  }

  const outlet = document.getElementById('note-outlet');
  if (outlet) {
    outlet.hidden = true;
    outlet.dataset.wbMode = '';
    delete outlet.dataset.note;
    delete outlet.dataset.layer;
  }
  const msgEl = document.getElementById('note-outlet-message');
  if (msgEl) {
    msgEl.hidden = true;
    msgEl.textContent = '';
  }
  const panel = document.getElementById('md-panel');
  if (panel) panel.hidden = false;
  if (docList) docList.style.display = '';
  if (dateHeading) {
    dateHeading.style.display = (date || state.ui.activeDate) ? '' : 'none';
  }
  if (date && typeof selectDate === 'function') selectDate(date);
}
