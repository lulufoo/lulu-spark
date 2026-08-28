import { renderAttachmentEditor } from '../ui/attachments.tsx';
import {
  addPlanAttachment,
  deletePlanAttachment,
  listPlanAttachments,
  pickLocalMarkdownFile,
  readPlanAttachment,
  savePlanAttachment,
  stagePlanAttachmentSource,
} from '../state/host.ts';
import {
  elementValue,
  errMessage,
  type AttachmentEditor,
  type TodoAttachment,
  type TodoPageCtx,
} from '../state/types.ts';

const ATTACHMENT_PICK_CANCEL_MSG = 'File selection cancelled';

export function createAttachmentsOwner(ctx: TodoPageCtx) {
  let attachments: TodoAttachment[] = [];
  let attachmentsError = '';
  let attachmentDeleteConfirm = '';
  let attachmentEditor: AttachmentEditor | null = null;
  let attachmentLoadToken = 0;

  function setAttachmentEditor(next: AttachmentEditor | null) {
    attachmentEditor = next;
  }

  function closeEditor() {
    attachmentLoadToken += 1;
    setAttachmentEditor(null);
  }

  function resetForSelectionChange() {
    attachments = [];
    attachmentsError = '';
    attachmentDeleteConfirm = '';
    closeEditor();
  }

  function refreshEditorNode(editorNode: Element, ui: { disabled?: boolean }) {
    if (!attachmentEditor) return false;
    const tmp = document.createElement('div');
    tmp.innerHTML = renderAttachmentEditor(attachmentEditor, ui.disabled);
    const fresh = tmp.firstElementChild;
    if (!fresh) return false;
    editorNode.innerHTML = fresh.innerHTML;
    for (const attr of [...fresh.attributes]) {
      editorNode.setAttribute(attr.name, attr.value);
    }
    return true;
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

  async function openEditor(fileName: string) {
    const selectedMasterId = ctx.getSelectedMasterId();
    if (!selectedMasterId || ctx.isBusy() || !fileName) return;
    const masterTaskId = selectedMasterId;
    const loadToken = ++attachmentLoadToken;
    setAttachmentEditor({
      fileName,
      content: '',
      editMode: false,
      error: '',
      loading: true,
    });
    ctx.setBusy(true);
    ctx.paint();
    try {
      const result = await readPlanAttachment({ masterTaskId, fileName });
      if (
        ctx.isDisposed() ||
        ctx.getSelectedMasterId() !== masterTaskId ||
        loadToken !== attachmentLoadToken
      ) {
        return;
      }
      const content =
        typeof result === 'string' ? result : String(result?.content ?? '');
      setAttachmentEditor({
        fileName,
        content,
        editMode: false,
        error: '',
        loading: false,
      });
    } catch (err) {
      if (
        ctx.isDisposed() ||
        ctx.getSelectedMasterId() !== masterTaskId ||
        loadToken !== attachmentLoadToken
      ) {
        return;
      }
      setAttachmentEditor({
        fileName,
        content: '',
        editMode: false,
        error: errMessage(err, 'Failed to load attachment'),
        loading: false,
      });
    } finally {
      if (loadToken === attachmentLoadToken) {
        ctx.setBusy(false);
        if (!ctx.isDisposed()) ctx.paint();
      }
    }
  }

  function enterEditMode() {
    if (!attachmentEditor || attachmentEditor.loading || ctx.isBusy()) return;
    setAttachmentEditor({ ...attachmentEditor, editMode: true, error: '' });
    ctx.paint();
  }

  function cancelEdit() {
    if (!attachmentEditor) return;
    setAttachmentEditor({ ...attachmentEditor, editMode: false, error: '' });
    ctx.paint();
  }

  async function saveEditor() {
    const selectedMasterId = ctx.getSelectedMasterId();
    if (!attachmentEditor || !selectedMasterId || ctx.isBusy()) return;
    const fileName = attachmentEditor.fileName;
    const editorEl = ctx.getContainer().querySelector('.todo-task-attachment-edit-area');
    const next = elementValue(editorEl);
    const content = next || attachmentEditor.content;
    setAttachmentEditor({ ...attachmentEditor, content, error: '' });
    ctx.setBusy(true);
    ctx.paint();
    try {
      const staged = await stagePlanAttachmentSource({
        preferredName: fileName,
        content,
      });
      const sourcePath = staged?.source_path || staged?.sourcePath;
      if (!sourcePath) {
        throw new Error('Failed to stage attachment');
      }
      await savePlanAttachment({
        masterTaskId: selectedMasterId,
        fileName,
        sourcePath,
      });
      if (ctx.isDisposed()) return;
      setAttachmentEditor({
        fileName,
        content,
        editMode: false,
        error: '',
        loading: false,
      });
    } catch (err) {
      if (ctx.isDisposed()) return;
      setAttachmentEditor({
        fileName,
        content,
        editMode: true,
        error: errMessage(err, 'Save failed'),
        loading: false,
      });
    } finally {
      ctx.setBusy(false);
      if (!ctx.isDisposed()) ctx.paint();
    }
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
    getEditor() {
      return attachmentEditor;
    },
    get deleteConfirm() {
      return attachmentDeleteConfirm;
    },
    resetForSelectionChange,
    closeEditor,
    clearOnListError() {
      attachments = [];
      attachmentsError = '';
    },
    loadForSelected,
    refreshEditorNode,
    appendEditor(container: Element, ui: { disabled?: boolean }, existingEditor: Element | null) {
      if (!attachmentEditor) return;
      const node = existingEditor || container.querySelector('.todo-task-attachment-editor');
      if (node && refreshEditorNode(node, ui)) {
        container.appendChild(node);
        return;
      }
      container.insertAdjacentHTML(
        'beforeend',
        renderAttachmentEditor(attachmentEditor, ui.disabled),
      );
    },
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
        void openEditor(fileName);
        return true;
      }
      if (action === 'edit-attachment') {
        enterEditMode();
        return true;
      }
      if (action === 'save-attachment') {
        void saveEditor();
        return true;
      }
      if (action === 'cancel-attachment-edit') {
        cancelEdit();
        return true;
      }
      if (action === 'close-attachment-editor') {
        closeEditor();
        ctx.paint();
        return true;
      }
      return false;
    },
    handleKeydown(event: KeyboardEvent) {
      if (event.key === 'Escape') {
        if (attachmentDeleteConfirm) {
          event.preventDefault();
          cancelDeleteConfirm();
          return true;
        }
        if (attachmentEditor) {
          event.preventDefault();
          if (attachmentEditor.loading) {
            closeEditor();
            ctx.setBusy(false);
            ctx.paint();
            return true;
          }
          if (ctx.isBusy()) return true;
          if (attachmentEditor.editMode) {
            cancelEdit();
            return true;
          }
          closeEditor();
          ctx.paint();
          return true;
        }
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
        void openEditor(fileName);
        return true;
      }
      return false;
    },
  };
  return api;
}
