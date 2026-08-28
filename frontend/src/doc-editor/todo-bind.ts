import { todosDocKey } from './identity.ts';
import { applyCachedHighlights, initDocHighlightOverlay } from './highlights.ts';
import type { TodoHighlightEditor } from './types.ts';

function previewEls(container: Element) {
  if (!container) return [];
  return [
    container.querySelector('.todo-task-plan-md-preview'),
    container.querySelector('.todo-task-attachment-preview'),
  ].filter((el): el is Element => Boolean(el));
}

function identityForBody(
  bodyEl: Element | null | undefined,
  taskId: string,
  attachmentEditor?: TodoHighlightEditor | null,
) {
  if (!taskId || !bodyEl) return '';
  if (
    bodyEl.classList.contains('todo-task-attachment-preview') &&
    attachmentEditor?.fileName
  ) {
    return todosDocKey(taskId, `att:${attachmentEditor.fileName}`);
  }
  return todosDocKey(taskId);
}

export function bindTodoDocHighlights(
  container: Element | null | undefined,
  taskId: string | null | undefined,
  attachmentEditor?: TodoHighlightEditor | null,
) {
  if (!container || !taskId) return;
  initDocHighlightOverlay({
    getBody: () => {
      const sel = window.getSelection();
      const node = sel?.anchorNode;
      const previews = previewEls(container);
      if (node) {
        const hit = previews.find((el) => el.contains(node));
        if (hit) return hit;
      }
      return previews[0] || null;
    },
    getEditArea: () =>
      container.querySelector('.todo-task-plan-md-editor') ||
      container.querySelector('.todo-task-attachment-edit-area'),
    getIdentityKey: (bodyEl) => identityForBody(bodyEl, taskId, attachmentEditor),
  });
  const plan = container.querySelector('.todo-task-plan-md-preview');
  if (plan) {
    void applyCachedHighlights({
      bodyEl: plan,
      identityKey: todosDocKey(taskId),
    });
  }
  const att = container.querySelector('.todo-task-attachment-preview');
  if (att && attachmentEditor?.fileName) {
    void applyCachedHighlights({
      bodyEl: att,
      identityKey: todosDocKey(taskId, `att:${attachmentEditor.fileName}`),
    });
  }
}
