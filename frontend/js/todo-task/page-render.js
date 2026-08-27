import { escHtml } from '../shared/utils.js';
import { renderPageHeader } from './list.js';

export const UNAVAILABLE_MSG = 'List temporarily unavailable. Please try again later.';
export const REFRESH_WARNING_MSG = 'Saved, but list refresh failed — retry';

export function renderDetailEmpty() {
  return `
    <div class="todo-task-split-detail-empty todo-task-empty">
      <p class="todo-task-empty-title">Select a todo on the left</p>
      <p class="todo-task-empty-detail">Or create a todo from the top right</p>
    </div>
  `;
}

export function renderDeadLink() {
  return `
    <div class="todo-task-split-dead-link todo-task-split-state">
      <p class="todo-task-split-state-title">Task not found</p>
      <p class="todo-task-split-state-detail">Link may be stale — pick again from the list</p>
    </div>
  `;
}

export function renderErrorEmpty(message = UNAVAILABLE_MSG) {
  return `
    <div class="todo-task-split-error todo-task-split-state todo-task-split-state--error">
      <p class="todo-task-split-state-title">Temporarily unavailable</p>
      <p class="todo-task-split-state-detail">${escHtml(message)}</p>
    </div>
  `;
}

export function renderPageShell({
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
