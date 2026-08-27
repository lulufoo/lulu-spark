import { todosDocKey } from './identity.js';
import { applyCachedHighlights, initDocHighlightOverlay } from './highlights.js';

function previewEls(container) {
  if (!container) return [];
  return [
    container.querySelector('.todo-task-plan-md-preview'),
    container.querySelector('.todo-task-attachment-preview'),
  ].filter(Boolean);
}

function identityForBody(bodyEl, taskId, attachmentEditor) {
  if (!taskId || !bodyEl) return '';
  if (
    bodyEl.classList.contains('todo-task-attachment-preview') &&
    attachmentEditor?.fileName
  ) {
    return todosDocKey(taskId, `att:${attachmentEditor.fileName}`);
  }
  return todosDocKey(taskId);
}

export function bindTodoDocHighlights(container, taskId, attachmentEditor) {
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
