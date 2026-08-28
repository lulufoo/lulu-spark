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
import type { TodoMaster } from '../state/types.ts';

type PageEventsDeps = {
  getBusy: () => boolean;
  paint: () => void;
  findMaster: (id: string) => TodoMaster | null;
  getSelectedMasterId: () => string;
  getSelectedSubId: () => string;
  setSelection: (next: { masterId?: string; subId?: string; deadLink?: boolean }) => void;
  getList: () => {
    filterCategoryId: string;
    toggleActiveOnly: () => void;
    setFilterCategoryId: (id: string) => void;
  };
  navigate?: (hash: string) => void;
  closeSubMenus: () => void;
  resetOwnersForMasterChange: () => void;
  syncTodosBindingForSelection: (masterId: string) => void;
  loadAttachmentsAndComments: () => Promise<void>;
  isDisposed: () => boolean;
  refresh: () => Promise<void>;
  setRefreshWarning: (value: string) => void;
  planMd: { handleClick: (action: string) => boolean };
  attachments: {
    handleClick: (event: Event, action: string, actionEl: HTMLElement | null) => boolean;
    handleKeydown: (event: KeyboardEvent) => boolean;
  };
  comments: {
    handleClick: (event: Event, action: string, actionEl: HTMLElement | null) => boolean;
    handleKeydown: (event: KeyboardEvent) => boolean;
  };
  detail: {
    handleClick: (event: Event, action: string, actionEl: HTMLElement | null) => boolean;
    handleInput: (event: Event) => boolean | void;
    handleBlur: (event: Event) => boolean | void;
    handleChange: (event: Event) => boolean | void;
    handleTitleEnter: (event: KeyboardEvent) => boolean | void;
  };
  dialogs: {
    openCreateDialog: (triggerEl: HTMLElement | null) => void;
    openAddSubDialog: (triggerEl: HTMLElement | null) => void;
    openDeleteMasterDialog: (triggerEl: HTMLElement | null) => void;
    openDeleteSubDialog: (triggerEl: HTMLElement | null, subTaskId: string, subTitle: string) => void;
    openCreateCategoryDialog: (triggerEl: EventTarget | null) => void;
    deleteFilteredCategory: () => Promise<void>;
    runMasterCategoryChange: (targetCategoryId: string, selectEl: HTMLSelectElement) => Promise<void>;
  };
  enforceActiveOnlySelection: () => void;
};

export function bindFocusRefresh(refresh: () => Promise<void> | void) {
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

export function bindPageEvents(container: HTMLElement, deps: PageEventsDeps) {
  const {
    getBusy,
    paint,
    findMaster,
    getSelectedMasterId,
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

  const onClick = (event: MouseEvent) => {
    const actionEl = (event.target as Element | null)?.closest(
      '[data-action]',
    ) as HTMLElement | null;
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

    const masterBtn = (event.target as Element | null)?.closest(
      '.todo-task-master-item',
    ) as HTMLElement | null;
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

    const clickTarget = event.target as Element | null;
    const subEl = clickTarget?.closest('.todo-task-sub') as HTMLElement | null;
    if (subEl?.dataset.subId && getSelectedMasterId()) {
      if (controlsDisabled(getBusy())) return;
      if (clickTarget?.closest('.todo-task-sub-menu')) return;
      if (clickTarget?.closest('.todo-task-sub-title-input')) return;
      if (clickTarget?.closest('.todo-task-sub-status-select')) return;
      if (clickTarget?.closest('.todo-task-sub-content-editor')) return;
      if (clickTarget?.closest('[data-action="toggle-sub-content"]')) return;
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

    if (!clickTarget?.closest('.todo-task-sub-menu')) {
      closeSubMenus();
    }
  };

  const onInput = (event: Event) => {
    detail.handleInput(event);
  };

  const onKeydown = (event: KeyboardEvent) => {
    if (comments.handleKeydown(event)) return;
    if (attachments.handleKeydown(event)) return;
    detail.handleTitleEnter(event);
  };

  const onFieldBlur = (event: Event) => {
    detail.handleBlur(event);
  };

  const onChange = (event: Event) => {
    const list = getList();
    const categoryFilter = (event.target as Element | null)?.closest(
      '[data-action="filter-category"]',
    );
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
    const masterCategorySelect = (event.target as Element | null)?.closest(
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
