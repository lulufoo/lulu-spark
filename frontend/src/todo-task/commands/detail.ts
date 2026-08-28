import {
  abandonPlanSub,
  completePlan,
  setPlanMasterStatus,
  updatePlanMasterTitle,
  updatePlanSub,
} from '../state/host.ts';
import { masterStatusClass } from '../state/format.ts';
import { errMessage, type TodoPageCtx } from '../state/types.ts';

export function createDetailOwner(ctx: TodoPageCtx) {
  const optimisticSubStatus: Record<string, string> = {};
  const subActionErrors: Record<string, string> = {};
  const subTitleErrors: Record<string, string> = {};
  const subTitleDrafts: Record<string, string> = {};
  const expandedSubContent: Record<string, boolean> = {};
  const subContentDrafts: Record<string, string> = {};
  let masterTitleDraft = '';
  let masterTitleError = '';

  function clearSubTitleState() {
    for (const key of Object.keys(subTitleDrafts)) delete subTitleDrafts[key];
    for (const key of Object.keys(subTitleErrors)) delete subTitleErrors[key];
  }

  function clearSubActionState() {
    for (const key of Object.keys(optimisticSubStatus)) delete optimisticSubStatus[key];
    for (const key of Object.keys(subActionErrors)) delete subActionErrors[key];
  }

  function resetForSelectionChange() {
    clearSubActionState();
    clearSubTitleState();
    masterTitleDraft = '';
    masterTitleError = '';
  }

  async function runSubStatusAction(
    subTaskId: string,
    targetStatus: string,
    actionFn: (args: { masterTaskId?: string; subTaskId?: string }) => Promise<unknown>,
  ) {
    const master = ctx.findSelectedMaster();
    const sub = master?.sub_tasks?.find((item) => item.sub_task_id === subTaskId);
    if (!master || !sub || sub.status !== 'incomplete') return;
    delete subActionErrors[subTaskId];
    optimisticSubStatus[subTaskId] = targetStatus;
    ctx.setBusy(true);
    ctx.paint();
    try {
      await actionFn({ masterTaskId: ctx.getSelectedMasterId(), subTaskId });
      delete optimisticSubStatus[subTaskId];
      ctx.setBusy(false);
      await ctx.reloadList({ afterWrite: true });
    } catch (err) {
      delete optimisticSubStatus[subTaskId];
      ctx.setBusy(false);
      subActionErrors[subTaskId] = errMessage(err, 'Operation failed');
      ctx.paint();
    }
  }

  async function runSubTitleSave(subTaskId: string, title: string) {
    const master = ctx.findSelectedMaster();
    const sub = master?.sub_tasks?.find((item) => item.sub_task_id === subTaskId);
    if (!master || !sub) return;
    const trimmed = title.trim();
    if (!trimmed) {
      subTitleErrors[subTaskId] = 'Title cannot be empty';
      ctx.paint();
      return;
    }
    if (trimmed === (sub.title || sub.sub_task_id)) {
      delete subTitleDrafts[subTaskId];
      delete subTitleErrors[subTaskId];
      return;
    }
    delete subTitleErrors[subTaskId];
    subTitleDrafts[subTaskId] = trimmed;
    ctx.setBusy(true);
    ctx.paint();
    try {
      await updatePlanSub({
        masterTaskId: ctx.getSelectedMasterId(),
        subTaskId,
        title: trimmed,
      });
      delete subTitleDrafts[subTaskId];
      ctx.setBusy(false);
      await ctx.reloadList({ afterWrite: true });
    } catch (err) {
      delete subTitleDrafts[subTaskId];
      ctx.setBusy(false);
      subTitleErrors[subTaskId] = errMessage(err, 'Save failed');
      ctx.paint();
    }
  }

  async function runSubContentSave(subTaskId: string, content: string) {
    const master = ctx.findSelectedMaster();
    const sub = master?.sub_tasks?.find((item) => item.sub_task_id === subTaskId);
    if (!master || !sub) return;
    const next = typeof content === 'string' ? content : '';
    const prev = typeof sub.content === 'string' ? sub.content : '';
    if (next === prev) {
      delete subContentDrafts[subTaskId];
      return;
    }
    subContentDrafts[subTaskId] = next;
    delete subActionErrors[subTaskId];
    ctx.setBusy(true);
    ctx.paint();
    try {
      await updatePlanSub({
        masterTaskId: ctx.getSelectedMasterId(),
        subTaskId,
        title: sub.title || sub.sub_task_id,
        content: next,
      });
      delete subContentDrafts[subTaskId];
      ctx.setBusy(false);
      await ctx.reloadList({ afterWrite: true });
    } catch (err) {
      ctx.setBusy(false);
      subActionErrors[subTaskId] = errMessage(err, 'Save failed');
      ctx.paint();
    }
  }

  async function runMasterTitleSave(title: string) {
    const master = ctx.findSelectedMaster();
    if (!master) return;
    const trimmed = title.trim();
    if (!trimmed) {
      masterTitleError = 'Title cannot be empty';
      ctx.paint();
      return;
    }
    if (trimmed === master.title) {
      masterTitleDraft = '';
      masterTitleError = '';
      return;
    }
    masterTitleError = '';
    masterTitleDraft = trimmed;
    ctx.setBusy(true);
    ctx.paint();
    try {
      await updatePlanMasterTitle({
        masterTaskId: ctx.getSelectedMasterId(),
        title: trimmed,
      });
      masterTitleDraft = '';
      ctx.setBusy(false);
      await ctx.reloadList({ afterWrite: true });
    } catch (err) {
      ctx.setBusy(false);
      masterTitleError = errMessage(err, 'Save failed');
      ctx.paint();
    }
  }

  async function runSubStatusChange(
    subTaskId: string,
    targetStatus: string,
    selectEl: HTMLSelectElement,
  ) {
    const master = ctx.findSelectedMaster();
    const sub = master?.sub_tasks?.find((item) => item.sub_task_id === subTaskId);
    const priorStatus = sub?.status ?? 'incomplete';
    if (!master || !sub || sub.status !== 'incomplete') {
      if (selectEl instanceof HTMLSelectElement) selectEl.value = priorStatus;
      return;
    }
    if (targetStatus === priorStatus) return;
    if (targetStatus !== 'complete' && targetStatus !== 'abandoned') {
      if (selectEl instanceof HTMLSelectElement) selectEl.value = priorStatus;
      return;
    }
    try {
      if (targetStatus === 'complete') {
        await runSubStatusAction(subTaskId, 'complete', completePlan);
      } else {
        await runSubStatusAction(subTaskId, 'abandoned', abandonPlanSub);
      }
    } catch {
      if (selectEl instanceof HTMLSelectElement) selectEl.value = priorStatus;
    }
  }

  async function runMasterStatusChange(targetStatus: string, selectEl: HTMLSelectElement) {
    const master = ctx.findSelectedMaster();
    const priorStatus = masterStatusClass(master?.status);
    if (!master) {
      if (selectEl instanceof HTMLSelectElement) selectEl.value = priorStatus;
      return;
    }
    if (
      targetStatus !== 'incomplete' &&
      targetStatus !== 'complete' &&
      targetStatus !== 'abandoned'
    ) {
      if (selectEl instanceof HTMLSelectElement) selectEl.value = priorStatus;
      return;
    }
    if (targetStatus === priorStatus) return;
    ctx.setBusy(true);
    ctx.paint();
    try {
      await setPlanMasterStatus({
        masterTaskId: ctx.getSelectedMasterId(),
        status: targetStatus,
      });
      ctx.setBusy(false);
      await ctx.reloadList({ afterWrite: true });
    } catch {
      ctx.setBusy(false);
      if (selectEl instanceof HTMLSelectElement) selectEl.value = priorStatus;
      ctx.paint();
    }
  }

  return {
    uiSlice() {
      return {
        subStatus: optimisticSubStatus,
        subActionErrors,
        subTitleErrors,
        subTitleDrafts,
        expandedSubContent,
        subContentDrafts,
        masterTitleDraft: masterTitleDraft || undefined,
        masterTitleError,
      };
    },
    resetForSelectionChange,
    setMasterTitleDraft(value: string) {
      masterTitleDraft = value;
    },
    setSubTitleDraft(subTaskId: string, value: string) {
      subTitleDrafts[subTaskId] = value;
    },
    setSubContentDraft(subTaskId: string, value: string) {
      subContentDrafts[subTaskId] = value;
    },
    toggleSubContent(subTaskId: string) {
      if (expandedSubContent[subTaskId]) delete expandedSubContent[subTaskId];
      else expandedSubContent[subTaskId] = true;
    },
    handleClick(event: Event, action: string, actionEl: HTMLElement | null) {
      if (action === 'toggle-sub-content') {
        event.stopPropagation();
        if (ctx.isBusy()) return true;
        const subTaskId = actionEl?.dataset.subId;
        if (!subTaskId) return true;
        this.toggleSubContent(subTaskId);
        ctx.paint();
        return true;
      }
      return false;
    },
    handleInput(event: Event) {
      const target = event.target as Element | null;
      const masterTitleInput = target?.closest('[data-action="edit-master-title"]');
      if (masterTitleInput instanceof HTMLInputElement) {
        if (ctx.isBusy() || !ctx.getSelectedMasterId()) return true;
        masterTitleDraft = masterTitleInput.value;
        return true;
      }
      const contentInput = target?.closest('[data-action="edit-sub-content"]');
      if (contentInput instanceof HTMLTextAreaElement) {
        if (ctx.isBusy() || !ctx.getSelectedMasterId()) return true;
        subContentDrafts[contentInput.dataset.subId ?? ''] = contentInput.value;
        return true;
      }
      const input = target?.closest('[data-action="edit-sub-title"]');
      if (!(input instanceof HTMLInputElement) || ctx.isBusy() || !ctx.getSelectedMasterId()) {
        return Boolean(input);
      }
      subTitleDrafts[input.dataset.subId ?? ''] = input.value;
      return true;
    },
    handleBlur(event: Event) {
      const target = event.target as Element | null;
      const masterTitleInput = target?.closest('[data-action="edit-master-title"]');
      if (masterTitleInput instanceof HTMLInputElement) {
        if (ctx.isBusy() || !ctx.getSelectedMasterId()) return true;
        void runMasterTitleSave(masterTitleInput.value);
        return true;
      }
      const contentInput = target?.closest('[data-action="edit-sub-content"]');
      if (contentInput instanceof HTMLTextAreaElement) {
        if (ctx.isBusy() || !ctx.getSelectedMasterId()) return true;
        const subTaskId = contentInput.dataset.subId;
        if (!subTaskId) return true;
        void runSubContentSave(subTaskId, contentInput.value);
        return true;
      }
      const input = target?.closest('[data-action="edit-sub-title"]');
      if (!(input instanceof HTMLInputElement) || ctx.isBusy() || !ctx.getSelectedMasterId()) {
        return Boolean(input instanceof HTMLInputElement);
      }
      const subTaskId = input.dataset.subId;
      if (!subTaskId) return true;
      void runSubTitleSave(subTaskId, input.value);
      return true;
    },
    handleChange(event: Event) {
      const target = event.target as Element | null;
      const masterSelect = target?.closest('[data-action="change-master-status"]');
      if (masterSelect instanceof HTMLSelectElement) {
        if (ctx.isBusy() || !ctx.getSelectedMasterId()) {
          const master = ctx.findSelectedMaster();
          masterSelect.value = masterStatusClass(master?.status);
          return true;
        }
        void runMasterStatusChange(masterSelect.value, masterSelect);
        return true;
      }
      const select = target?.closest('[data-action="change-sub-status"]');
      if (!(select instanceof HTMLSelectElement)) return false;
      if (ctx.isBusy() || !ctx.getSelectedMasterId()) {
        select.value = select.dataset.currentStatus ?? select.value;
        return true;
      }
      const subTaskId = select.dataset.subId;
      if (!subTaskId) return true;
      void runSubStatusChange(subTaskId, select.value, select);
      return true;
    },
    handleTitleEnter(event: KeyboardEvent) {
      const titleInput = (event.target as Element | null)?.closest(
        '[data-action="edit-master-title"], [data-action="edit-sub-title"]',
      );
      if (!(titleInput instanceof HTMLInputElement)) return false;
      if (event.key === 'Enter') {
        event.preventDefault();
        titleInput.blur();
        return true;
      }
      return false;
    },
  };
}
