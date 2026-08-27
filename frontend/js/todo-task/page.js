import { closeTodoTaskDialog } from './dialog.js';
import { createTodosPageLifecycle } from './lifecycle.js';
import {
  getTauriListen,
  listPlanCategories,
  loadTodoTasks,
} from './host.js';
import {
  controlsDisabled,
  filterMastersForView,
  isIncompleteMaster,
  masterCategoryId,
  pickDefaultSub,
  sortMasters,
} from './format.js';
import { createListOwner, renderMasterPane, syncCategoryFilterWidth } from './list.js';
import { createDetailOwner, renderSubDetailPane } from './detail.js';
import { createPlanMdOwner } from './plan-md.js';
import { createAttachmentsOwner, renderAttachmentEditor } from './attachments.js';
import { createCommentsOwner } from './comments.js';
import { bindTodoDocHighlights } from '../doc-editor/index.js';
import { createPageDialogs } from './page-dialogs.js';
import { bindPageEvents } from './page-events.js';
import {
  REFRESH_WARNING_MSG,
  bindFocusRefresh,
  renderDeadLink,
  renderDetailEmpty,
  renderErrorEmpty,
  renderPageShell,
} from './page-render.js';

const AI_ASSISTANT_TURN_COMPLETED = 'ai-assistant:turn-completed';

/**
 * @param {HTMLElement} container
 * @param {{ masterId?: string, subId?: string, navigate?: (hash: string) => void }} [opts]
 */
export function mountTodoTaskSplit(container, opts = {}) {
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

  container.innerHTML = '<div class="todo-task-split-loading">Loading…</div>';

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

  function renderDetailPane() {
    if (deadLink) return renderDeadLink();
    if (!selectedMasterId) return renderDetailEmpty();
    const master = findMaster(selectedMasterId);
    if (!master) return renderDeadLink();
    return renderSubDetailPane(master, selectedSubId, getUi());
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
    const masterHtml = renderMasterPane(
      masters,
      selectedMasterId,
      ui.disabled,
      list.activeOnly,
      list.filterCategoryId,
    );
    const existingEditor = container.querySelector('.todo-task-attachment-editor');
    if (existingEditor) existingEditor.remove();
    container.innerHTML = renderPageShell({
      masterHtml,
      detailHtml: renderDetailPane(),
      disabled: ui.disabled,
      activeOnly: list.activeOnly,
      categories,
      filterCategoryId: list.filterCategoryId,
      categoryError: list.categoryError,
    });
    paintedMasterId = selectedMasterId;
    const nextMaster = container.querySelector('.todo-task-split-master');
    const nextDetail = container.querySelector('.todo-task-split-detail');
    if (nextMaster) nextMaster.scrollTop = masterScroll;
    if (nextDetail && keepDetailScroll) nextDetail.scrollTop = detailScroll;
    syncCategoryFilterWidth(container);
    if (attachments.getEditor()) {
      if (existingEditor && attachments.refreshEditorNode(existingEditor, ui)) {
        container.appendChild(existingEditor);
      } else {
        container.insertAdjacentHTML(
          'beforeend',
          renderAttachmentEditor(attachments.getEditor(), ui.disabled),
        );
      }
    }
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
      container.innerHTML = renderPageShell({
        masterHtml: '<div class="todo-task-split-state"></div>',
        detailHtml: renderErrorEmpty(),
        categories,
        filterCategoryId: list.filterCategoryId,
        categoryError: list.categoryError,
      });
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
      container.innerHTML = '';
    }
  }

  dispose.applyRoute = applyRoute;
  dispose.refresh = refresh;
  return { dispose, unmount: dispose, refresh, applyRoute };
}
