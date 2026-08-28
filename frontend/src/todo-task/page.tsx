// @ts-nocheck
import { createElement, useLayoutEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { closeTodoTaskDialog } from './commands/dialog.ts';
import { createTodosPageLifecycle } from './commands/lifecycle.ts';
import {
  getTauriListen,
  listPlanCategories,
  loadTodoTasks,
} from './state/host.ts';
import {
  controlsDisabled,
  filterMastersForView,
  isIncompleteMaster,
  masterCategoryId,
  pickDefaultSub,
  sortMasters,
} from './state/format.ts';
import { createListOwner } from './commands/list.ts';
import { MasterPane, syncCategoryFilterWidth } from './ui/list.tsx';
import { createDetailOwner } from './commands/detail.ts';
import { SubDetailPane } from './ui/detail.tsx';
import { createPlanMdOwner } from './commands/plan-md.ts';
import { createAttachmentsOwner } from './commands/attachments.ts';
import { createCommentsOwner } from './commands/comments.ts';
import { bindTodoDocHighlights } from '../doc-editor/index.ts';
import { createPageDialogs } from './commands/page-dialogs.ts';
import { bindFocusRefresh, bindPageEvents } from './commands/page-events.ts';
import {
  DeadLink,
  DetailEmpty,
  ErrorEmpty,
  PageShell,
  REFRESH_WARNING_MSG,
} from './ui/page-render.tsx';

const AI_ASSISTANT_TURN_COMPLETED = 'ai-assistant:turn-completed';

function loadingNode() {
  return createElement('div', { className: 'todo-task-split-loading' }, 'Loading…');
}

/**
 * Session for one Todos visit. `renderPage` paints JSX; this function does not
 * create a React root. Production uses TodoTasksPage; tests use mountTodoTaskSplit.
 *
 * @param {HTMLElement} container
 * @param {{ masterId?: string, subId?: string, navigate?: (hash: string) => void }} [opts]
 * @param {(node: unknown) => void} renderPage
 */
function startTodoTasksSession(container, opts, renderPage) {
  const { masterId: initialMasterId = '', subId: initialSubId = '', navigate } = opts;
  let disposed = false;
  let masters = [];
  let selectedMasterId = initialMasterId;
  let selectedSubId = initialSubId;
  let deadLink = false;
  let validateInitialSubLink = Boolean(initialSubId);
  let refreshPromise = null;
  let busy = false;
  let refreshWarning = '';
  let paintedMasterId = '';
  /** @type {Array<{ id: string, name: string, is_default?: boolean }>} */
  let categories = [];
  let paintTick = 0;

  const ctx = {
    isDisposed: () => disposed,
    isBusy: () => controlsDisabled(busy),
    setBusy: (value) => {
      busy = value;
    },
    paint: () => paint(),
    getSelectedMasterId: () => selectedMasterId,
    getContainer: () => container,
    findSelectedMaster: () => findMaster(selectedMasterId),
    reloadList: (options) => reloadList(options),
    getCategories: () => categories,
  };

  const list = createListOwner();
  const detail = createDetailOwner(ctx);
  const planMd = createPlanMdOwner(ctx);
  const attachments = createAttachmentsOwner(ctx);
  const comments = createCommentsOwner(ctx);

  const todosLifecycle = createTodosPageLifecycle({
    onUnbound: () => {
      if (!disposed) paint();
    },
  });
  let lifecycleEntered = false;

  function commitPage(node) {
    flushSync(() => {
      renderPage(node);
    });
  }

  commitPage(loadingNode());

  function syncTodosBindingForSelection(masterId) {
    if (disposed) return;
    const id = typeof masterId === 'string' ? masterId.trim() : '';
    if (!lifecycleEntered) {
      lifecycleEntered = true;
      void todosLifecycle.onTodosPageEnter(id);
      return;
    }
    if (!id) return;
    void todosLifecycle.onMasterSelectionChange(id);
  }

  function findMaster(id) {
    return masters.find((master) => master.master_task_id === id) ?? null;
  }

  function setSelection({ masterId, subId, deadLink: nextDead }) {
    if (masterId !== undefined) selectedMasterId = masterId;
    if (subId !== undefined) selectedSubId = subId;
    if (nextDead !== undefined) deadLink = nextDead;
  }

  function getUi() {
    return {
      disabled: controlsDisabled(busy),
      refreshWarning,
      ...planMd.uiSlice(),
      ...attachments.uiSlice(),
      ...comments.uiSlice(),
      ...detail.uiSlice(),
      categories,
      categoryError: list.categoryError,
      filterCategoryId: list.filterCategoryId,
    };
  }

  function closeSubMenus() {
    container.querySelectorAll('.todo-task-sub-menu-panel').forEach((panel) => {
      panel.hidden = true;
    });
  }

  function clearSelectionSideState() {
    selectedMasterId = '';
    selectedSubId = '';
    deadLink = false;
    detail.resetForSelectionChange();
    attachments.resetForSelectionChange();
    comments.resetForSelectionChange();
  }

  function enforceActiveOnlySelection() {
    if (!selectedMasterId) return;
    const master = findMaster(selectedMasterId);
    if (!master) return;
    if (list.activeOnly && !isIncompleteMaster(master)) {
      clearSelectionSideState();
      return;
    }
    if (list.filterCategoryId && masterCategoryId(master) !== list.filterCategoryId) {
      clearSelectionSideState();
    }
  }

  function resolveSelection() {
    deadLink = false;
    if (!selectedMasterId) {
      selectedSubId = '';
    } else {
      const master = findMaster(selectedMasterId);
      if (!master) {
        deadLink = true;
        selectedSubId = '';
      } else {
        const subs = master.sub_tasks ?? [];
        if (validateInitialSubLink && initialSubId && selectedSubId === initialSubId) {
          validateInitialSubLink = false;
          const sub = subs.find((item) => item.sub_task_id === selectedSubId);
          if (!sub) {
            deadLink = true;
            selectedSubId = '';
          }
        } else if (
          !selectedSubId ||
          !subs.some((item) => item.sub_task_id === selectedSubId)
        ) {
          const fallback = pickDefaultSub(master);
          selectedSubId = fallback?.sub_task_id ?? '';
        }
      }
    }
    enforceActiveOnlySelection();
  }

  function selectDefaultMasterOnEnter() {
    if (initialMasterId || selectedMasterId) return;
    const firstVisibleMaster = sortMasters(
      filterMastersForView(masters, list.activeOnly, list.filterCategoryId),
    )[0];
    if (!firstVisibleMaster) return;
    selectedMasterId = firstVisibleMaster.master_task_id;
    selectedSubId = pickDefaultSub(firstVisibleMaster)?.sub_task_id ?? '';
  }

  function renderDetailNode() {
    if (deadLink) {
      return createElement(DeadLink);
    }
    if (!selectedMasterId) {
      return createElement(DetailEmpty);
    }
    const master = findMaster(selectedMasterId);
    if (!master) {
      return createElement(DeadLink);
    }
    return createElement(SubDetailPane, { master, selectedSubId, ui: getUi() });
  }

  function paint() {
    if (!masters.length && container.querySelector('.todo-task-split-error')) {
      return;
    }
    const masterPane = container.querySelector('.todo-task-split-master');
    const detailPane = container.querySelector('.todo-task-split-detail');
    const masterScroll = masterPane?.scrollTop ?? 0;
    const detailScroll = detailPane?.scrollTop ?? 0;
    const keepDetailScroll =
      Boolean(selectedMasterId) && selectedMasterId === paintedMasterId;
    const ui = getUi();
    const existingEditor = container.querySelector('.todo-task-attachment-editor');
    if (existingEditor) existingEditor.remove();
    paintTick += 1;
    commitPage(
      createElement(PageShell, {
        key: paintTick,
        disabled: ui.disabled,
        activeOnly: list.activeOnly,
        categories,
        filterCategoryId: list.filterCategoryId,
        categoryError: list.categoryError,
        master: createElement(MasterPane, {
          masters,
          selectedMasterId,
          disabled: ui.disabled,
          activeOnly: list.activeOnly,
          categoryId: list.filterCategoryId,
        }),
        detail: renderDetailNode(),
      }),
    );
    paintedMasterId = selectedMasterId;
    const nextMaster = container.querySelector('.todo-task-split-master');
    const nextDetail = container.querySelector('.todo-task-split-detail');
    if (nextMaster) nextMaster.scrollTop = masterScroll;
    if (nextDetail && keepDetailScroll) nextDetail.scrollTop = detailScroll;
    syncCategoryFilterWidth(container);
    attachments.appendEditor(container, ui, existingEditor);
    bindTodoDocHighlights(container, selectedMasterId, attachments.getEditor());
  }

  function resetOwnersForMasterChange() {
    closeSubMenus();
    planMd.reset();
    attachments.resetForSelectionChange();
    comments.resetForSelectionChange();
    detail.resetForSelectionChange();
  }

  function applyRoute(route = {}) {
    if (disposed) return;
    const masterId = route.masterId ?? '';
    const subId = route.subId ?? '';
    if (masterId === selectedMasterId && subId === selectedSubId) return;
    const masterChanged = masterId !== selectedMasterId;
    resetOwnersForMasterChange();
    selectedMasterId = masterId;
    selectedSubId = subId;
    validateInitialSubLink = Boolean(subId);
    deadLink = false;
    resolveSelection();
    if (masterChanged) {
      syncTodosBindingForSelection(selectedMasterId);
    }
    void (async () => {
      await attachments.loadForSelected();
      await comments.loadForSelected();
      if (disposed) return;
      paint();
    })();
  }

  async function loadCategories() {
    try {
      categories = await listPlanCategories();
    } catch {
      categories = [];
    }
  }

  async function reloadList({ afterWrite = false } = {}) {
    try {
      const [entries] = await Promise.all([loadTodoTasks(), loadCategories()]);
      if (disposed) return;
      masters = entries;
      resolveSelection();
      if (!lifecycleEntered) {
        selectDefaultMasterOnEnter();
      }
      if (afterWrite) {
        refreshWarning = '';
      }
      await attachments.loadForSelected();
      await comments.loadForSelected();
      if (disposed) return;
      paint();
      if (!lifecycleEntered) {
        syncTodosBindingForSelection(selectedMasterId);
      }
    } catch {
      if (disposed) return;
      if (afterWrite) {
        refreshWarning = REFRESH_WARNING_MSG;
        paint();
        return;
      }
      masters = [];
      selectedMasterId = '';
      selectedSubId = '';
      attachments.clearOnListError();
      comments.clearOnListError();
      refreshWarning = '';
      commitPage(
        createElement(PageShell, {
          key: ++paintTick,
          categories,
          filterCategoryId: list.filterCategoryId,
          categoryError: list.categoryError,
          master: createElement('div', { className: 'todo-task-split-state' }),
          detail: createElement(ErrorEmpty),
        }),
      );
      if (!lifecycleEntered) {
        syncTodosBindingForSelection('');
      }
    }
  }

  async function refresh() {
    if (refreshPromise) {
      return refreshPromise;
    }
    refreshPromise = reloadList().finally(() => {
      refreshPromise = null;
    });
    return refreshPromise;
  }

  async function runWriteAction(actionFn) {
    busy = true;
    paint();
    try {
      await actionFn();
      busy = false;
      await reloadList({ afterWrite: true });
    } catch (err) {
      busy = false;
      throw err;
    }
  }

  function onAiAssistantTurnCompleted(event) {
    if (disposed) return;
    const payload = event?.payload;
    if (!payload || payload.wrote !== true) return;
    void reloadList({ afterWrite: true });
  }

  const dialogs = createPageDialogs({
    getBusy: () => busy,
    setBusy: (value) => {
      busy = value;
    },
    paint,
    findMaster,
    getSelectedMasterId: () => selectedMasterId,
    getSelectedSubId: () => selectedSubId,
    setSelection,
    getCategories: () => categories,
    getList: () => list,
    isDisposed: () => disposed,
    reloadList,
    loadCategories,
    runWriteAction,
    navigate,
    syncTodosBindingForSelection,
  });

  const disposeEvents = bindPageEvents(container, {
    getBusy: () => busy,
    paint,
    findMaster,
    getSelectedMasterId: () => selectedMasterId,
    getSelectedSubId: () => selectedSubId,
    setSelection,
    getList: () => list,
    navigate,
    closeSubMenus,
    resetOwnersForMasterChange,
    syncTodosBindingForSelection,
    loadAttachmentsAndComments: async () => {
      await attachments.loadForSelected();
      await comments.loadForSelected();
    },
    isDisposed: () => disposed,
    refresh,
    setRefreshWarning: (value) => {
      refreshWarning = value;
    },
    planMd,
    attachments,
    comments,
    detail,
    dialogs,
    enforceActiveOnlySelection,
  });

  const onDialogClose = () => {
    if (!disposed) paint();
  };
  document.addEventListener('todo-task-dialog-close', onDialogClose);
  const disposeFocusRefresh = bindFocusRefresh(refresh);

  let unlistenTurnCompleted = null;
  const listen = getTauriListen();
  if (listen) {
    void listen(AI_ASSISTANT_TURN_COMPLETED, onAiAssistantTurnCompleted).then(
      (unlisten) => {
        if (disposed) {
          if (typeof unlisten === 'function') void unlisten();
          return;
        }
        unlistenTurnCompleted = unlisten;
      },
    );
  }

  void refresh();

  function dispose() {
    if (disposed) return;
    disposed = true;
    try {
      void todosLifecycle.onTodosPageLeave();
    } finally {
      document.removeEventListener('todo-task-dialog-close', onDialogClose);
      closeTodoTaskDialog();
      disposeFocusRefresh();
      if (typeof unlistenTurnCompleted === 'function') {
        void unlistenTurnCompleted();
        unlistenTurnCompleted = null;
      }
      disposeEvents();
    }
  }

  return { dispose, refresh, applyRoute };
}

export type TodoTasksPageProps = {
  masterId?: string;
  subId?: string;
  navigate?: (hash: string) => void;
};

/** Todos page. Production child of ShellPages; does not createRoot the page slot. */
export function TodoTasksPage({
  masterId = '',
  subId = '',
  navigate,
}: TodoTasksPageProps = {}) {
  const hostRef = useRef(null);
  const sessionRef = useRef(null);
  const [view, setView] = useState(loadingNode);

  useLayoutEffect(() => {
    const host = hostRef.current;
    if (!host) return undefined;
    const session = startTodoTasksSession(host, { masterId, subId, navigate }, setView);
    sessionRef.current = session;
    return () => {
      session.dispose();
      sessionRef.current = null;
    };
    // Start once for this visit. Hash master/sub updates go through applyRoute.
  }, []);

  useLayoutEffect(() => {
    sessionRef.current?.applyRoute({ masterId, subId });
  }, [masterId, subId]);

  return createElement('div', { ref: hostRef, className: 'todo-tasks-react-host' }, view);
}

/**
 * Test / leftover helper. Production Todos is a child of ShellPages, not this root.
 *
 * @param {HTMLElement} container
 * @param {{ masterId?: string, subId?: string, navigate?: (hash: string) => void }} [opts]
 */
export function mountTodoTaskSplit(container, opts = {}) {
  const reactRoot = createRoot(container);
  const session = startTodoTasksSession(container, opts, (node) => {
    reactRoot.render(node);
  });

  function dispose() {
    session.dispose();
    flushSync(() => {
      reactRoot.unmount();
    });
    container.innerHTML = '';
  }

  dispose.applyRoute = session.applyRoute;
  dispose.refresh = session.refresh;
  return { dispose, unmount: dispose, refresh: session.refresh, applyRoute: session.applyRoute };
}
