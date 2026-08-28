// @ts-nocheck
import {
  COPY_MASTER_LABEL,
  buildDeepLink,
  controlsDisabled,
  flashCopyFeedback,
  masterCategoryId,
  pickDefaultSub,
} from '../state/format.ts';
import { CATEGORY_ACTION_CREATE, CATEGORY_ACTION_DELETE } from './list.ts';
import { DEFAULT_PLAN_CATEGORY_ID } from '../state/host.ts';

export function bindFocusRefresh(refresh) {
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

export function bindPageEvents(container, deps) {
  const {
    getBusy,
    paint,
    findMaster,
    getSelectedMasterId,
    getSelectedSubId,
    setSelection,
    getList,
    navigate,
    closeSubMenus,
    resetOwnersForMasterChange,
    syncTodosBindingForSelection,
    loadAttachmentsAndComments,
    isDisposed,
    refresh,
    setRefreshWarning,
    planMd,
    attachments,
    comments,
    detail,
    dialogs,
  } = deps;

  const onClick = (event) => {
    const actionEl = event.target.closest('[data-action]');
    const action = actionEl?.dataset.action;

    if (action === 'retry-refresh') {
      event.preventDefault();
      if (controlsDisabled(getBusy())) return;
      setRefreshWarning('');
      void refresh();
      return;
    }

    if (action && planMd.handleClick(action)) {
      event.preventDefault();
      return;
    }

    if (action === 'toggle-active-only') {
      event.preventDefault();
      if (controlsDisabled(getBusy())) return;
      getList().toggleActiveOnly();
      deps.enforceActiveOnlySelection();
      paint();
      return;
    }

    if (action === 'create-master') {
      event.preventDefault();
      if (controlsDisabled(getBusy())) return;
      dialogs.openCreateDialog(actionEl instanceof HTMLElement ? actionEl : null);
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
      if (controlsDisabled(getBusy()) || !getSelectedMasterId()) return;
      dialogs.openAddSubDialog(actionEl instanceof HTMLElement ? actionEl : null);
      return;
    }

    if (action === 'delete-master') {
      event.preventDefault();
      if (controlsDisabled(getBusy()) || !getSelectedMasterId()) return;
      dialogs.openDeleteMasterDialog(actionEl instanceof HTMLElement ? actionEl : null);
      return;
    }

    if (action === 'delete-sub') {
      event.preventDefault();
      event.stopPropagation();
      closeSubMenus();
      if (controlsDisabled(getBusy()) || !getSelectedMasterId()) return;
      const subTaskId = actionEl?.dataset.subId;
      const subTitle = actionEl?.dataset.subTitle ?? '';
      if (!subTaskId) return;
      dialogs.openDeleteSubDialog(
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
      if (controlsDisabled(getBusy())) return;
      if (masterBtn.dataset.masterId === getSelectedMasterId()) {
        closeSubMenus();
        return;
      }
      resetOwnersForMasterChange();
      const selectedMasterId = masterBtn.dataset.masterId;
      const master = findMaster(selectedMasterId);
      const fallback = master ? pickDefaultSub(master) : null;
      setSelection({
        masterId: selectedMasterId,
        subId: fallback?.sub_task_id ?? '',
        deadLink: false,
      });
      syncTodosBindingForSelection(selectedMasterId);
      void (async () => {
        await loadAttachmentsAndComments();
        if (isDisposed()) return;
        paint();
        if (typeof navigate === 'function' && selectedMasterId && fallback?.sub_task_id) {
          navigate(buildDeepLink(selectedMasterId, fallback.sub_task_id));
        }
      })();
      return;
    }

    const subEl = event.target.closest('.todo-task-sub');
    if (subEl?.dataset.subId && getSelectedMasterId()) {
      if (controlsDisabled(getBusy())) return;
      if (event.target.closest('.todo-task-sub-menu')) return;
      if (event.target.closest('.todo-task-sub-title-input')) return;
      if (event.target.closest('.todo-task-sub-status-select')) return;
      if (event.target.closest('.todo-task-sub-content-editor')) return;
      if (event.target.closest('[data-action="toggle-sub-content"]')) return;
      closeSubMenus();
      setSelection({
        masterId: getSelectedMasterId(),
        subId: subEl.dataset.subId,
        deadLink: false,
      });
      paint();
      if (typeof navigate === 'function') {
        navigate(buildDeepLink(getSelectedMasterId(), subEl.dataset.subId));
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
    const list = getList();
    const categoryFilter = event.target.closest('[data-action="filter-category"]');
    if (categoryFilter instanceof HTMLSelectElement) {
      if (controlsDisabled(getBusy())) {
        categoryFilter.value = list.filterCategoryId;
        return;
      }
      const nextValue = categoryFilter.value || '';
      if (nextValue === CATEGORY_ACTION_CREATE) {
        categoryFilter.value = list.filterCategoryId;
        dialogs.openCreateCategoryDialog(categoryFilter);
        return;
      }
      if (nextValue === CATEGORY_ACTION_DELETE) {
        categoryFilter.value = list.filterCategoryId;
        void dialogs.deleteFilteredCategory();
        return;
      }
      list.setFilterCategoryId(nextValue);
      deps.enforceActiveOnlySelection();
      paint();
      return;
    }
    const masterCategorySelect = event.target.closest(
      '[data-action="change-master-category"]',
    );
    if (masterCategorySelect instanceof HTMLSelectElement) {
      if (controlsDisabled(getBusy()) || !getSelectedMasterId()) {
        const master = findMaster(getSelectedMasterId());
        masterCategorySelect.value = master
          ? masterCategoryId(master)
          : DEFAULT_PLAN_CATEGORY_ID;
        return;
      }
      void dialogs.runMasterCategoryChange(masterCategorySelect.value, masterCategorySelect);
      return;
    }
    detail.handleChange(event);
  };

  container.addEventListener('click', onClick);
  container.addEventListener('input', onInput);
  container.addEventListener('keydown', onKeydown);
  container.addEventListener('focusout', onFieldBlur);
  container.addEventListener('change', onChange);

  return () => {
    container.removeEventListener('click', onClick);
    container.removeEventListener('input', onInput);
    container.removeEventListener('keydown', onKeydown);
    container.removeEventListener('focusout', onFieldBlur);
    container.removeEventListener('change', onChange);
  };
}
