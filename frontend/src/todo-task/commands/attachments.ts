import { todosDocKey } from '../../doc-editor/identity.ts';
import { openFilePopup } from '../../file-popup/index.ts';
import {
  addPlanAttachment,
  deletePlanAttachment,
  listPlanAttachments,
  pickLocalMarkdownFile,
  stagePlanAttachmentSource,
} from '../state/host.ts';
import { errMessage, type TodoAttachment, type TodoPageCtx } from '../state/types.ts';

const ATTACHMENT_PICK_CANCEL_MSG = 'File selection cancelled';

export function createAttachmentsOwner(ctx: TodoPageCtx) {
  let attachments: TodoAttachment[] = [];
  let attachmentsError = '';
  let attachmentDeleteConfirm = '';

  function resetForSelectionChange() {
    attachments = [];
    attachmentsError = '';
    attachmentDeleteConfirm = '';
  }

  async function loadForSelected() {
    const selectedMasterId = ctx.getSelectedMasterId();
    if (!selectedMasterId) {
      attachments = [];
      return;
    }
    const masterId = selectedMasterId;
    try {
      const entries = await listPlanAttachments({ masterTaskId: masterId });
      if (ctx.isDisposed() || ctx.getSelectedMasterId() !== masterId) return;
      attachments = entries;
    } catch {
      if (ctx.isDisposed() || ctx.getSelectedMasterId() !== masterId) return;
      attachments = [];
    }
  }

  function openAttachment(fileName: string, pathHint?: string) {
    const selectedMasterId = ctx.getSelectedMasterId();
    if (!selectedMasterId || ctx.isBusy() || !fileName) return;
    const entry = attachments.find((item) => item.file_name === fileName);
    const path = String(pathHint || entry?.path || '').trim();
    if (!path) return;
    openFilePopup({
      path,
      title: fileName,
      identityKey: todosDocKey(selectedMasterId, `att:${fileName}`),
    });
  }

  async function pickAndAdd() {
    const selectedMasterId = ctx.getSelectedMasterId();
    if (!selectedMasterId || ctx.isBusy()) return;
    attachmentsError = '';
    ctx.paint();
    let picked;
    try {
      picked = await pickLocalMarkdownFile();
    } catch (err) {
      const pickMsg = errMessage(err, '');
      attachmentsError = pickMsg
        ? `Failed to pick file: ${pickMsg}`
        : 'Failed to pick file';
      ctx.paint();
      return;
    }
    if (!picked) {
      attachmentsError = ATTACHMENT_PICK_CANCEL_MSG;
      ctx.paint();
      return;
    }
    ctx.setBusy(true);
    ctx.paint();
    try {
      const staged = await stagePlanAttachmentSource({
        preferredName: picked.fileName,
        content: picked.content,
      });
      const sourcePath = staged?.source_path || staged?.sourcePath;
      if (!sourcePath) {
        throw new Error('Failed to stage attachment');
      }
      await addPlanAttachment({
        masterTaskId: selectedMasterId,
        sourcePath,
      });
      attachmentsError = '';
      ctx.setBusy(false);
      await loadForSelected();
      if (!ctx.isDisposed()) ctx.paint();
    } catch (err) {
      ctx.setBusy(false);
      attachmentsError = errMessage(err, 'Failed to add attachment');
      ctx.paint();
    }
  }

  function openDeleteConfirm(fileName: string) {
    if (!ctx.getSelectedMasterId() || ctx.isBusy() || !fileName) return;
    attachmentDeleteConfirm = fileName;
    attachmentsError = '';
    ctx.paint();
  }

  function cancelDeleteConfirm() {
    attachmentDeleteConfirm = '';
    ctx.paint();
  }

  async function confirmDelete(fileName: string) {
    const selectedMasterId = ctx.getSelectedMasterId();
    if (!selectedMasterId || ctx.isBusy() || !fileName) return;
    const masterTaskId = selectedMasterId;
    ctx.setBusy(true);
    attachmentsError = '';
    ctx.paint();
    try {
      await deletePlanAttachment({ masterTaskId, fileName });
      if (ctx.isDisposed() || ctx.getSelectedMasterId() !== masterTaskId) return;
      attachmentDeleteConfirm = '';
      attachmentsError = '';
      await loadForSelected();
    } catch (err) {
      if (ctx.isDisposed() || ctx.getSelectedMasterId() !== masterTaskId) return;
      attachmentDeleteConfirm = '';
      attachmentsError = errMessage(err, 'Failed to delete attachment');
    } finally {
      ctx.setBusy(false);
      if (!ctx.isDisposed()) ctx.paint();
    }
  }

  const api = {
    uiSlice() {
      return { attachments, attachmentsError, attachmentDeleteConfirm };
    },
    get deleteConfirm() {
      return attachmentDeleteConfirm;
    },
    resetForSelectionChange,
    clearOnListError() {
      attachments = [];
      attachmentsError = '';
    },
    loadForSelected,
    handleClick(event: Event, action: string, actionEl: HTMLElement | null) {
      if (action === 'pick-attachment-md') {
        void pickAndAdd();
        return true;
      }
      if (action === 'delete-attachment') {
        event.stopPropagation();
        const fileName = actionEl?.dataset.fileName;
        if (!fileName) return true;
        openDeleteConfirm(fileName);
        return true;
      }
      if (action === 'confirm-delete-attachment') {
        event.stopPropagation();
        const fileName = actionEl?.dataset.fileName || attachmentDeleteConfirm;
        if (!fileName) return true;
        void confirmDelete(fileName);
        return true;
      }
      if (action === 'cancel-delete-attachment') {
        event.stopPropagation();
        cancelDeleteConfirm();
        return true;
      }
      if (action === 'open-attachment') {
        const fileName = actionEl?.dataset.fileName;
        if (!fileName) return true;
        openAttachment(fileName, actionEl?.dataset.path);
        return true;
      }
      return false;
    },
    handleKeydown(event: KeyboardEvent) {
      if (event.key === 'Escape' && attachmentDeleteConfirm) {
        event.preventDefault();
        cancelDeleteConfirm();
        return true;
      }
      const attachItem = (event.target as Element | null)?.closest(
        '[data-action="open-attachment"]',
      );
      if (
        attachItem instanceof HTMLElement &&
        (event.key === 'Enter' || event.key === ' ')
      ) {
        event.preventDefault();
        if (ctx.isBusy() || !ctx.getSelectedMasterId()) return true;
        const fileName = attachItem.dataset.fileName;
        if (!fileName) return true;
        openAttachment(fileName, attachItem.dataset.path);
        return true;
      }
      return false;
    },
  };
  return api;
}
