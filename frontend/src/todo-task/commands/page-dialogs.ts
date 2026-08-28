import { closeTodoTaskDialog, openTodoTaskDialog } from './dialog.ts';
import {
  addPlanSub,
  createPlanCategory,
  createTodoTask,
  deletePlanCategory,
  deletePlanSub,
  deleteTodoTask,
  setPlanCategory,
} from '../state/host.ts';
import {
  buildMasterDeepLink,
  controlsDisabled,
  isDefaultCategory,
  masterCategoryId,
} from '../state/format.ts';
import { DEFAULT_PLAN_CATEGORY_ID } from '../state/host.ts';
import { errMessage, type TodoCategory, type TodoMaster } from '../state/types.ts';

type PageDialogsDeps = {
  getBusy: () => boolean;
  setBusy: (value: boolean) => void;
  paint: () => void;
  findMaster: (id: string) => TodoMaster | null;
  getSelectedMasterId: () => string;
  getSelectedSubId: () => string;
  setSelection: (next: { masterId?: string; subId?: string; deadLink?: boolean }) => void;
  getCategories: () => TodoCategory[];
  getList: () => {
    filterCategoryId: string;
    resetError: () => void;
    setCategoryError: (message: string) => void;
    clearFilterCategory: () => void;
  };
  isDisposed: () => boolean;
  reloadList: (options?: { afterWrite?: boolean }) => Promise<void>;
  loadCategories: () => Promise<void>;
  runWriteAction: (actionFn: () => Promise<void>) => Promise<void>;
  navigate?: (hash: string) => void;
  syncTodosBindingForSelection: (masterId: string) => void;
};

export function createPageDialogs(deps: PageDialogsDeps) {
  const {
    getBusy,
    setBusy,
    paint,
    findMaster,
    getSelectedMasterId,
    getSelectedSubId,
    setSelection,
    getCategories,
    getList,
    isDisposed,
    reloadList,
    loadCategories,
    runWriteAction,
    navigate,
    syncTodosBindingForSelection,
  } = deps;

  function openCreateCategoryDialog(triggerEl: EventTarget | null) {
    if (controlsDisabled(getBusy())) return;
    getList().resetError();
    openTodoTaskDialog({
      type: 'create-category',
      triggerEl: triggerEl instanceof HTMLElement ? triggerEl : null,
      onSubmit: async ({ name }: { name?: unknown }) => {
        const trimmed = typeof name === 'string' ? name.trim() : '';
        if (!trimmed) {
          throw new Error('Please enter a category name');
        }
        await createPlanCategory({ name: trimmed });
        await loadCategories();
        if (!isDisposed()) paint();
      },
    });
  }

  async function deleteFilteredCategory() {
    const list = getList();
    if (controlsDisabled(getBusy()) || !list.filterCategoryId) return;
    const selected = getCategories().find((category) => category.id === list.filterCategoryId);
    if (isDefaultCategory(selected, list.filterCategoryId)) return;
    list.resetError();
    setBusy(true);
    paint();
    try {
      await deletePlanCategory({ categoryId: list.filterCategoryId });
      list.clearFilterCategory();
      await loadCategories();
      setBusy(false);
      await reloadList({ afterWrite: true });
    } catch (err) {
      setBusy(false);
      list.setCategoryError(errMessage(err, 'Failed to delete category'));
      if (!isDisposed()) paint();
    }
  }

  async function runMasterCategoryChange(
    targetCategoryId: string,
    selectEl: HTMLSelectElement,
  ) {
    const selectedMasterId = getSelectedMasterId();
    const master = findMaster(selectedMasterId);
    const prior = master ? masterCategoryId(master) : DEFAULT_PLAN_CATEGORY_ID;
    if (!master || !selectedMasterId) {
      if (selectEl instanceof HTMLSelectElement) selectEl.value = prior;
      return;
    }
    if (!targetCategoryId || targetCategoryId === prior) return;
    getList().resetError();
    setBusy(true);
    paint();
    try {
      await setPlanCategory({
        masterTaskId: selectedMasterId,
        categoryId: targetCategoryId,
      });
      setBusy(false);
      await reloadList({ afterWrite: true });
    } catch (err) {
      setBusy(false);
      getList().setCategoryError(errMessage(err, 'Failed to update category'));
      if (selectEl instanceof HTMLSelectElement) {
        selectEl.value = prior;
      }
      if (!isDisposed()) paint();
    }
  }

  function openCreateDialog(triggerEl: HTMLElement | null) {
    openTodoTaskDialog({
      type: 'create-master',
      triggerEl,
      onSubmit: async ({ title, subTitles }: { title?: string; subTitles?: string[] }) => {
        await runWriteAction(async () => {
          const result = await createTodoTask({ title, subTitles });
          const createdId = result?.master_task_id ?? result?.task?.master_task_id;
          if (createdId) {
            setSelection({ masterId: createdId, subId: '', deadLink: false });
            syncTodosBindingForSelection(createdId);
          }
        });
      },
    });
  }

  function openAddSubDialog(triggerEl: HTMLElement | null) {
    const master = findMaster(getSelectedMasterId());
    if (!master) return;
    openTodoTaskDialog({
      type: 'add-sub',
      triggerEl,
      payload: { masterTitle: master.title },
      onSubmit: async ({ title }: { title?: string }) => {
        const masterTaskId = getSelectedMasterId();
        await runWriteAction(async () => {
          await addPlanSub({ masterTaskId, title });
        });
      },
    });
  }

  function openDeleteMasterDialog(triggerEl: HTMLElement | null) {
    const master = findMaster(getSelectedMasterId());
    if (!master) return;
    openTodoTaskDialog({
      type: 'delete-master',
      triggerEl,
      payload: {
        masterTitle: master.title,
        subCount: master.sub_tasks?.length ?? 0,
      },
      onSubmit: async () => {
        const masterTaskId = getSelectedMasterId();
        await runWriteAction(async () => {
          await deleteTodoTask({ masterTaskId });
          setSelection({ masterId: '', subId: '', deadLink: false });
          if (typeof navigate === 'function') {
            navigate('#/todo-tasks');
          }
        });
      },
    });
  }

  function openDeleteSubDialog(
    triggerEl: HTMLElement | null,
    subTaskId: string,
    subTitle: string,
  ) {
    openTodoTaskDialog({
      type: 'delete-sub',
      triggerEl,
      payload: { subTitle },
      onSubmit: async () => {
        const masterTaskId = getSelectedMasterId();
        const deletingSelected = getSelectedSubId() === subTaskId;
        await runWriteAction(async () => {
          await deletePlanSub({ masterTaskId, subTaskId });
          if (deletingSelected) {
            setSelection({ masterId: masterTaskId, subId: '', deadLink: false });
            if (typeof navigate === 'function' && masterTaskId) {
              navigate(buildMasterDeepLink(masterTaskId));
            }
          }
        });
      },
    });
  }

  return {
    openCreateCategoryDialog,
    deleteFilteredCategory,
    runMasterCategoryChange,
    openCreateDialog,
    openAddSubDialog,
    openDeleteMasterDialog,
    openDeleteSubDialog,
    closeTodoTaskDialog,
  };
}
