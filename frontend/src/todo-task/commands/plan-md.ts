// @ts-nocheck
import { readPlanMd, updatePlanMd } from '../state/host.ts';

export function createPlanMdOwner(ctx) {
  let planMdEditMode = false;
  let planMdDraft = '';
  let planMdError = '';
  let planMdLoading = false;

  function reset() {
    planMdEditMode = false;
    planMdDraft = '';
    planMdError = '';
    planMdLoading = false;
  }

  return {
    uiSlice() {
      return { planMdEditMode, planMdDraft, planMdError, planMdLoading };
    },
    reset,
    async enterEdit() {
      const selectedMasterId = ctx.getSelectedMasterId();
      if (!selectedMasterId || ctx.isBusy()) return;
      planMdLoading = true;
      planMdError = '';
      ctx.paint();
      try {
        planMdDraft = await readPlanMd({ masterTaskId: selectedMasterId });
        planMdEditMode = true;
      } catch (err) {
        planMdError = err?.message || 'Failed to load description';
        planMdEditMode = false;
      } finally {
        planMdLoading = false;
        ctx.paint();
      }
    },
    async save() {
      const selectedMasterId = ctx.getSelectedMasterId();
      if (!selectedMasterId || ctx.isBusy()) return;
      const editor = ctx.getContainer().querySelector('.todo-task-plan-md-editor');
      planMdDraft = editor instanceof HTMLTextAreaElement ? editor.value : planMdDraft;
      planMdError = '';
      ctx.setBusy(true);
      ctx.paint();
      try {
        await updatePlanMd({ masterTaskId: selectedMasterId, planMd: planMdDraft });
        reset();
        ctx.setBusy(false);
        await ctx.reloadList({ afterWrite: true });
      } catch (err) {
        ctx.setBusy(false);
        planMdEditMode = true;
        planMdError = err?.message || 'Save failed';
        ctx.paint();
      }
    },
    cancel() {
      if (ctx.isBusy()) return;
      reset();
      ctx.paint();
    },
    handleClick(action) {
      if (action === 'edit-plan-md') {
        void this.enterEdit();
        return true;
      }
      if (action === 'save-plan-md') {
        void this.save();
        return true;
      }
      if (action === 'cancel-plan-md') {
        this.cancel();
        return true;
      }
      return false;
    },
  };
}
