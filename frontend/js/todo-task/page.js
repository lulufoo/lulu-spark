import { escHtml } from '../utils.js';
import { closeTodoTaskDialog, openTodoTaskDialog } from './dialog.js';
import { createTodosPageLifecycle } from './todos-lifecycle.js';
import {
  DEFAULT_PLAN_CATEGORY_ID,
  addPlanSub,
  createPlanCategory,
  createTodoTask,
  deletePlanCategory,
  deletePlanSub,
  deleteTodoTask,
  getTauriListen,
  listPlanCategories,
  loadTodoTasks,
  setPlanCategory,
} from './host.js';
import {
  COPY_MASTER_LABEL,
  buildDeepLink,
  buildMasterDeepLink,
  controlsDisabled,
  filterMastersForView,
  flashCopyFeedback,
  isDefaultCategory,
  isIncompleteMaster,
  masterCategoryId,
  pickDefaultSub,
  sortMasters,
} from './format.js';
import {
  CATEGORY_ACTION_CREATE,
  CATEGORY_ACTION_DELETE,
  createListOwner,
  renderMasterPane,
  renderPageHeader,
  syncCategoryFilterWidth,
} from './list.js';
import { createDetailOwner, renderSubDetailPane } from './detail.js';
import { createPlanMdOwner } from './plan-md.js';
import { createAttachmentsOwner, renderAttachmentEditor } from './attachments.js';
import { createCommentsOwner } from './comments.js';

const UNAVAILABLE_MSG = 'List temporarily unavailable. Please try again later.';
const REFRESH_WARNING_MSG = 'Saved, but list refresh failed — retry';
const AI_ASSISTANT_TURN_COMPLETED = 'ai-assistant:turn-completed';

function renderDetailEmpty() {
  return `
    <div class="todo-task-split-detail-empty todo-task-empty">
      <p class="todo-task-empty-title">Select a todo on the left</p>
      <p class="todo-task-empty-detail">Or create a todo from the top right</p>
    </div>
  `;
}

function renderDeadLink() {
  return `
    <div class="todo-task-split-dead-link todo-task-split-state">
      <p class="todo-task-split-state-title">Task not found</p>
      <p class="todo-task-split-state-detail">Link may be stale — pick again from the list</p>
    </div>
  `;
}

function renderErrorEmpty(message = UNAVAILABLE_MSG) {
  return `
    <div class="todo-task-split-error todo-task-split-state todo-task-split-state--error">
      <p class="todo-task-split-state-title">Temporarily unavailable</p>
      <p class="todo-task-split-state-detail">${escHtml(message)}</p>
    </div>
  `;
}

function renderPageShell({
  masterHtml,
  detailHtml,
  disabled = false,
  activeOnly = true,
  categories = [],
  filterCategoryId = '',
  categoryError = '',
}) {
  return `
    <div class="todo-tasks-page">
      ${renderPageHeader(disabled, activeOnly, categories, filterCategoryId, categoryError)}
      <div class="todo-task-split">
        <aside class="todo-task-split-master" aria-label="Todos list">${masterHtml}</aside>
        <section class="todo-task-split-detail" aria-label="Task details">${detailHtml}</section>
      </div>
    </div>
  `;
}

function bindFocusRefresh(refresh) {
  const onFocus = () => {
    void refresh();
  };
  const onVisibility = () => {
    if (document.visibilityState === 'visible') {
      void refresh();
    }
  };
  window.addEventListener('focus', onFocus);
  document.addEventListener('visibilitychange', onVisibility);
  return () => {
    window.removeEventListener('focus', onFocus);
    document.removeEventListener('visibilitychange', onVisibility);
  };
}

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
    if (!attachments.getEditor()) return;
    if (existingEditor && attachments.refreshEditorNode(existingEditor, ui)) {
      container.appendChild(existingEditor);
      return;
    }
    container.insertAdjacentHTML(
      'beforeend',
      renderAttachmentEditor(attachments.getEditor(), ui.disabled),
    );
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

  function openCreateCategoryDialog(triggerEl) {
    if (controlsDisabled(busy)) return;
    list.resetError();
    openTodoTaskDialog({
      type: 'create-category',
      triggerEl: triggerEl instanceof HTMLElement ? triggerEl : null,
      onSubmit: async ({ name }) => {
        const trimmed = typeof name === 'string' ? name.trim() : '';
        if (!trimmed) {
          throw new Error('Please enter a category name');
        }
        await createPlanCategory({ name: trimmed });
        await loadCategories();
        if (!disposed) paint();
      },
    });
  }

  async function deleteFilteredCategory() {
    if (controlsDisabled(busy) || !list.filterCategoryId) return;
    const selected = categories.find((category) => category.id === list.filterCategoryId);
    if (isDefaultCategory(selected, list.filterCategoryId)) return;
    list.resetError();
    busy = true;
    paint();
    try {
      await deletePlanCategory({ categoryId: list.filterCategoryId });
      list.clearFilterCategory();
      await loadCategories();
      busy = false;
      await reloadList({ afterWrite: true });
    } catch (err) {
      busy = false;
      list.setCategoryError(err?.message || 'Failed to delete category');
      if (!disposed) paint();
    }
  }

  async function runMasterCategoryChange(targetCategoryId, selectEl) {
    const master = findMaster(selectedMasterId);
    const prior = master ? masterCategoryId(master) : DEFAULT_PLAN_CATEGORY_ID;
    if (!master || !selectedMasterId) {
      if (selectEl instanceof HTMLSelectElement) selectEl.value = prior;
      return;
    }
    if (!targetCategoryId || targetCategoryId === prior) return;
    list.resetError();
    busy = true;
    paint();
    try {
      await setPlanCategory({
        masterTaskId: selectedMasterId,
        categoryId: targetCategoryId,
      });
      busy = false;
      await reloadList({ afterWrite: true });
    } catch (err) {
      busy = false;
      list.setCategoryError(err?.message || 'Failed to update category');
      if (selectEl instanceof HTMLSelectElement) {
        selectEl.value = prior;
      }
      if (!disposed) paint();
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

  function openCreateDialog(triggerEl) {
    openTodoTaskDialog({
      type: 'create-master',
      triggerEl,
      onSubmit: async ({ title, subTitles }) => {
        await runWriteAction(async () => {
          const result = await createTodoTask({ title, subTitles });
          const createdId = result?.master_task_id ?? result?.task?.master_task_id;
          if (createdId) {
            selectedMasterId = createdId;
            selectedSubId = '';
            deadLink = false;
            syncTodosBindingForSelection(selectedMasterId);
          }
        });
      },
    });
  }

  function openAddSubDialog(triggerEl) {
    const master = findMaster(selectedMasterId);
    if (!master) return;
    openTodoTaskDialog({
      type: 'add-sub',
      triggerEl,
      payload: { masterTitle: master.title },
      onSubmit: async ({ title }) => {
        const masterTaskId = selectedMasterId;
        await runWriteAction(async () => {
          await addPlanSub({ masterTaskId, title });
        });
      },
    });
  }

  function openDeleteMasterDialog(triggerEl) {
    const master = findMaster(selectedMasterId);
    if (!master) return;
    openTodoTaskDialog({
      type: 'delete-master',
      triggerEl,
      payload: {
        masterTitle: master.title,
        subCount: master.sub_tasks?.length ?? 0,
      },
      onSubmit: async () => {
        const masterTaskId = selectedMasterId;
        await runWriteAction(async () => {
          await deleteTodoTask({ masterTaskId });
          selectedMasterId = '';
          selectedSubId = '';
          deadLink = false;
          if (typeof navigate === 'function') {
            navigate('#/todo-tasks');
          }
        });
      },
    });
  }

  function openDeleteSubDialog(triggerEl, subTaskId, subTitle) {
    openTodoTaskDialog({
      type: 'delete-sub',
      triggerEl,
      payload: { subTitle },
      onSubmit: async () => {
        const masterTaskId = selectedMasterId;
        const deletingSelected = selectedSubId === subTaskId;
        await runWriteAction(async () => {
          await deletePlanSub({ masterTaskId, subTaskId });
          if (deletingSelected) {
            selectedSubId = '';
            deadLink = false;
            if (typeof navigate === 'function' && masterTaskId) {
              navigate(buildMasterDeepLink(masterTaskId));
            }
          }
        });
      },
    });
  }

  const onClick = (event) => {
    const actionEl = event.target.closest('[data-action]');
    const action = actionEl?.dataset.action;

    if (action === 'retry-refresh') {
      event.preventDefault();
      if (controlsDisabled(busy)) return;
      refreshWarning = '';
      void refresh();
      return;
    }

    if (action && planMd.handleClick(action)) {
      event.preventDefault();
      return;
    }

    if (action === 'toggle-active-only') {
      event.preventDefault();
      if (controlsDisabled(busy)) return;
      list.toggleActiveOnly();
      enforceActiveOnlySelection();
      paint();
      return;
    }

    if (action === 'create-master') {
      event.preventDefault();
      if (controlsDisabled(busy)) return;
      openCreateDialog(actionEl instanceof HTMLElement ? actionEl : null);
      return;
    }

    if (action && (attachments.handleClick(event, action, actionEl) || comments.handleClick(event, action, actionEl))) {
      event.preventDefault();
      return;
    }

    if (action && detail.handleClick(event, action, actionEl)) {
      event.preventDefault();
      return;
    }

    if (action === 'add-sub') {
      event.preventDefault();
      if (controlsDisabled(busy) || !selectedMasterId) return;
      openAddSubDialog(actionEl instanceof HTMLElement ? actionEl : null);
      return;
    }

    if (action === 'delete-master') {
      event.preventDefault();
      if (controlsDisabled(busy) || !selectedMasterId) return;
      openDeleteMasterDialog(actionEl instanceof HTMLElement ? actionEl : null);
      return;
    }

    if (action === 'delete-sub') {
      event.preventDefault();
      event.stopPropagation();
      closeSubMenus();
      if (controlsDisabled(busy) || !selectedMasterId) return;
      const subTaskId = actionEl?.dataset.subId;
      const subTitle = actionEl?.dataset.subTitle ?? '';
      if (!subTaskId) return;
      openDeleteSubDialog(
        actionEl instanceof HTMLElement ? actionEl : null,
        subTaskId,
        subTitle,
      );
      return;
    }

    if (action === 'toggle-sub-menu') {
      event.preventDefault();
      event.stopPropagation();
      const panel = actionEl?.closest('.todo-task-sub-menu')?.querySelector('.todo-task-sub-menu-panel');
      if (!(panel instanceof HTMLElement)) return;
      const willOpen = panel.hidden;
      closeSubMenus();
      panel.hidden = !willOpen;
      return;
    }

    if (action === 'copy-sub-id' || action === 'copy-master-id') {
      event.preventDefault();
      event.stopPropagation();
      const text = actionEl?.dataset.copyText ?? '';
      if (text && navigator.clipboard?.writeText) {
        void navigator.clipboard.writeText(text).then(() => {
          if (action === 'copy-master-id' && actionEl instanceof HTMLElement) {
            flashCopyFeedback(actionEl, COPY_MASTER_LABEL);
          }
        });
      }
      closeSubMenus();
      return;
    }

    const masterBtn = event.target.closest('.todo-task-master-item');
    if (masterBtn?.dataset.masterId) {
      if (controlsDisabled(busy)) return;
      if (masterBtn.dataset.masterId === selectedMasterId) {
        closeSubMenus();
        return;
      }
      resetOwnersForMasterChange();
      selectedMasterId = masterBtn.dataset.masterId;
      const master = findMaster(selectedMasterId);
      const fallback = master ? pickDefaultSub(master) : null;
      selectedSubId = fallback?.sub_task_id ?? '';
      deadLink = false;
      syncTodosBindingForSelection(selectedMasterId);
      void (async () => {
        await attachments.loadForSelected();
        await comments.loadForSelected();
        if (disposed) return;
        paint();
        if (typeof navigate === 'function' && selectedMasterId && selectedSubId) {
          navigate(buildDeepLink(selectedMasterId, selectedSubId));
        }
      })();
      return;
    }

    const subEl = event.target.closest('.todo-task-sub');
    if (subEl?.dataset.subId && selectedMasterId) {
      if (controlsDisabled(busy)) return;
      if (event.target.closest('.todo-task-sub-menu')) return;
      if (event.target.closest('.todo-task-sub-title-input')) return;
      if (event.target.closest('.todo-task-sub-status-select')) return;
      if (event.target.closest('.todo-task-sub-content-editor')) return;
      if (event.target.closest('[data-action="toggle-sub-content"]')) return;
      closeSubMenus();
      selectedSubId = subEl.dataset.subId;
      deadLink = false;
      paint();
      if (typeof navigate === 'function') {
        navigate(buildDeepLink(selectedMasterId, selectedSubId));
      }
      return;
    }

    if (!event.target.closest('.todo-task-sub-menu')) {
      closeSubMenus();
    }
  };

  const onInput = (event) => {
    detail.handleInput(event);
  };

  const onKeydown = (event) => {
    if (comments.handleKeydown(event)) return;
    if (attachments.handleKeydown(event)) return;
    detail.handleTitleEnter(event);
  };

  const onFieldBlur = (event) => {
    detail.handleBlur(event);
  };

  const onChange = (event) => {
    const categoryFilter = event.target.closest('[data-action="filter-category"]');
    if (categoryFilter instanceof HTMLSelectElement) {
      if (controlsDisabled(busy)) {
        categoryFilter.value = list.filterCategoryId;
        return;
      }
      const nextValue = categoryFilter.value || '';
      if (nextValue === CATEGORY_ACTION_CREATE) {
        categoryFilter.value = list.filterCategoryId;
        openCreateCategoryDialog(categoryFilter);
        return;
      }
      if (nextValue === CATEGORY_ACTION_DELETE) {
        categoryFilter.value = list.filterCategoryId;
        void deleteFilteredCategory();
        return;
      }
      list.setFilterCategoryId(nextValue);
      enforceActiveOnlySelection();
      paint();
      return;
    }
    const masterCategorySelect = event.target.closest(
      '[data-action="change-master-category"]',
    );
    if (masterCategorySelect instanceof HTMLSelectElement) {
      if (controlsDisabled(busy) || !selectedMasterId) {
        const master = findMaster(selectedMasterId);
        masterCategorySelect.value = master
          ? masterCategoryId(master)
          : DEFAULT_PLAN_CATEGORY_ID;
        return;
      }
      void runMasterCategoryChange(masterCategorySelect.value, masterCategorySelect);
      return;
    }
    detail.handleChange(event);
  };

  container.addEventListener('click', onClick);
  container.addEventListener('input', onInput);
  container.addEventListener('keydown', onKeydown);
  container.addEventListener('focusout', onFieldBlur);
  container.addEventListener('change', onChange);
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
      container.removeEventListener('click', onClick);
      container.removeEventListener('input', onInput);
      container.removeEventListener('keydown', onKeydown);
      container.removeEventListener('focusout', onFieldBlur);
      container.removeEventListener('change', onChange);
      container.innerHTML = '';
    }
  }

  dispose.applyRoute = applyRoute;
  dispose.refresh = refresh;
  return { dispose, unmount: dispose, refresh, applyRoute };
}
