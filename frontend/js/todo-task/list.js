import { escHtml } from '../utils.js';
import {
  categoryDisplayName,
  filterMastersForView,
  formatRelativeTime,
  isDefaultCategory,
  masterStatusClass,
  sortMasters,
} from './format.js';

const ACTIVE_ONLY_LABEL = 'Active only';
const ACTIVE_ONLY_EMPTY_TITLE = 'No active todos';
const ACTIVE_ONLY_EMPTY_DETAIL =
  'Turn off Active only to see completed and abandoned todos.';
const ALL_CATEGORIES_LABEL = 'All categories';
export const CATEGORY_ACTION_CREATE = '__create_category__';
export const CATEGORY_ACTION_DELETE = '__delete_category__';
const CATEGORY_FILTER_EMPTY_TITLE = 'No todos in this category';
const CATEGORY_FILTER_EMPTY_DETAIL = 'Choose another category or create a todo in this one.';

function renderCategoryControls(categories, selectedCategoryId, categoryError, disabled) {
  const disabledAttr = disabled ? ' disabled' : '';
  const selected = (categories ?? []).find((category) => category.id === selectedCategoryId);
  const deleteDisabled =
    disabled || !selectedCategoryId || isDefaultCategory(selected, selectedCategoryId);
  const deleteDisabledAttr = deleteDisabled ? ' disabled' : '';
  const options = [
    `<option value="">${escHtml(ALL_CATEGORIES_LABEL)}</option>`,
    ...(categories ?? []).map((category) => {
      const selectedAttr = category.id === selectedCategoryId ? ' selected' : '';
      return `<option value="${escHtml(category.id)}"${selectedAttr}>${escHtml(categoryDisplayName(category))}</option>`;
    }),
    `<option value="${CATEGORY_ACTION_CREATE}"${disabled ? ' disabled' : ''}>+ Add category</option>`,
    `<option value="${CATEGORY_ACTION_DELETE}"${deleteDisabledAttr}>Delete category</option>`,
  ].join('');
  const errHtml = categoryError
    ? `<p class="todo-task-category-error" role="alert">${escHtml(categoryError)}</p>`
    : '';
  return `
    <div class="todo-task-category-controls">
      <select
        class="todo-task-category-filter"
        data-action="filter-category"
        aria-label="Filter by category"${disabledAttr}
      >${options}</select>
      ${errHtml}
    </div>
  `;
}

function measureOptionLabelWidth(text, font) {
  const label = String(text ?? '');
  if (typeof document !== 'undefined') {
    const probe = document.createElement('span');
    probe.style.cssText = [
      'position:absolute',
      'visibility:hidden',
      'pointer-events:none',
      'white-space:nowrap',
      `font:${font || '13px sans-serif'}`,
      'padding:0',
      'border:0',
    ].join(';');
    probe.textContent = label;
    document.body.appendChild(probe);
    const width = probe.offsetWidth;
    probe.remove();
    if (width > 0) return width;
  }
  // jsdom / no-layout fallback (~13px UI font)
  return label.length * 8;
}

export function syncCategoryFilterWidth(root) {
  const select = root?.querySelector?.('.todo-task-category-filter');
  if (!(select instanceof HTMLSelectElement)) return;
  const font = getComputedStyle(select).font || '13px sans-serif';
  let maxPx = 0;
  for (const opt of select.options) {
    maxPx = Math.max(maxPx, measureOptionLabelWidth(opt.textContent, font));
  }
  // Horizontal padding (12+26) + custom chevron affordance.
  const chromePx = 42;
  select.style.width = `${Math.ceil(maxPx + chromePx)}px`;
}

export function renderPageHeader(
  disabled,
  activeOnly = true,
  categories = [],
  selectedCategoryId = '',
  categoryError = '',
) {
  const disabledAttr = disabled ? ' disabled' : '';
  const checked = activeOnly ? 'true' : 'false';
  return `
    <header class="todo-tasks-page-header">
      <h1 class="todo-tasks-page-title">Todos</h1>
      <div class="todo-tasks-page-header-actions">
        ${renderCategoryControls(categories, selectedCategoryId, categoryError, disabled)}
        <label class="todo-task-active-only">
          <span class="todo-task-active-only-label">${ACTIVE_ONLY_LABEL}</span>
          <button
            type="button"
            class="todo-task-active-only-switch"
            role="switch"
            data-action="toggle-active-only"
            aria-checked="${checked}"
            aria-label="${ACTIVE_ONLY_LABEL}"${disabledAttr}
          ></button>
        </label>
        <button type="button" class="md-header-btn primary" data-action="create-master"${disabledAttr}>+ New todo</button>
      </div>
    </header>
  `;
}

function renderMasterList(masters, selectedMasterId, disabled) {
  const disabledAttr = disabled ? ' disabled' : '';
  const items = sortMasters(masters)
    .map((master) => {
      const selected =
        master.master_task_id === selectedMasterId ? ' todo-task-master-item--selected' : '';
      const statusMod = ` todo-task-master-item--${masterStatusClass(master.status)}`;
      const subCount = master.sub_tasks?.length ?? 0;
      const meta = `${subCount} sub-tasks · ${formatRelativeTime(master.created_at) || 'Unknown time'}`;
      return `
        <li>
          <button type="button" class="todo-task-master-item${selected}${statusMod}" data-master-id="${escHtml(master.master_task_id)}"${disabledAttr}>
            <span class="todo-task-master-title">${escHtml(master.title)}</span>
            <span class="todo-task-master-meta">${escHtml(meta)}</span>
          </button>
        </li>
      `;
    })
    .join('');
  return `<ul class="todo-task-master-list" role="list">${items}</ul>`;
}

function renderMasterEmpty(disabled) {
  const disabledAttr = disabled ? ' disabled' : '';
  return `
    <div class="todo-task-empty todo-task-empty--sidebar">
      <p class="todo-task-empty-title">No todos yet</p>
      <p class="todo-task-empty-detail">Create your first todo to manage sub-tasks</p>
      <button type="button" class="md-header-btn primary" data-action="create-master"${disabledAttr}>New todo</button>
    </div>
  `;
}

function renderActiveOnlyEmpty() {
  return `
    <div class="todo-task-empty todo-task-empty--sidebar">
      <p class="todo-task-empty-title">${ACTIVE_ONLY_EMPTY_TITLE}</p>
      <p class="todo-task-empty-detail">${ACTIVE_ONLY_EMPTY_DETAIL}</p>
    </div>
  `;
}

function renderCategoryFilterEmpty() {
  return `
    <div class="todo-task-empty todo-task-empty--sidebar">
      <p class="todo-task-empty-title">${CATEGORY_FILTER_EMPTY_TITLE}</p>
      <p class="todo-task-empty-detail">${CATEGORY_FILTER_EMPTY_DETAIL}</p>
    </div>
  `;
}

export function renderMasterPane(
  masters,
  selectedMasterId,
  disabled,
  activeOnly = true,
  categoryId = '',
) {
  const visible = filterMastersForView(masters, activeOnly, categoryId);
  if (!masters.length) {
    return renderMasterEmpty(disabled);
  }
  if (!visible.length) {
    if (categoryId && filterMastersForView(masters, activeOnly, '').length) {
      return renderCategoryFilterEmpty();
    }
    return renderActiveOnlyEmpty();
  }
  return renderMasterList(visible, selectedMasterId, disabled);
}

export function createListOwner() {
  let activeOnly = true;
  let filterCategoryId = '';
  let categoryError = '';

  return {
    get activeOnly() {
      return activeOnly;
    },
    get filterCategoryId() {
      return filterCategoryId;
    },
    get categoryError() {
      return categoryError;
    },
    setCategoryError(message) {
      categoryError = message || '';
    },
    toggleActiveOnly() {
      activeOnly = !activeOnly;
    },
    setFilterCategoryId(id) {
      filterCategoryId = id || '';
      categoryError = '';
    },
    clearFilterCategory() {
      filterCategoryId = '';
    },
    resetError() {
      categoryError = '';
    },
  };
}
