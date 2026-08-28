import { renderToHtml } from '../../island.ts';
import {
  categoryDisplayName,
  filterMastersForView,
  formatRelativeTime,
  isDefaultCategory,
  masterStatusClass,
  sortMasters,
} from '../state/format.ts';
import { CATEGORY_ACTION_CREATE, CATEGORY_ACTION_DELETE } from '../commands/list.ts';
import type { TodoCategory, TodoMaster } from '../state/types.ts';

const ACTIVE_ONLY_LABEL = 'Active only';
const ACTIVE_ONLY_EMPTY_TITLE = 'No active todos';
const ACTIVE_ONLY_EMPTY_DETAIL =
  'Turn off Active only to see completed and abandoned todos.';
const ALL_CATEGORIES_LABEL = 'All categories';
const CATEGORY_FILTER_EMPTY_TITLE = 'No todos in this category';
const CATEGORY_FILTER_EMPTY_DETAIL = 'Choose another category or create a todo in this one.';

function CategoryControls({
  categories,
  selectedCategoryId,
  categoryError,
  disabled,
}: {
  categories: TodoCategory[];
  selectedCategoryId: string;
  categoryError: string;
  disabled: boolean;
}) {
  const selected = (categories ?? []).find((category) => category.id === selectedCategoryId);
  const deleteDisabled =
    disabled || !selectedCategoryId || isDefaultCategory(selected, selectedCategoryId);
  return (
    <div className="todo-task-category-controls">
      <select
        className="todo-task-category-filter"
        data-action="filter-category"
        aria-label="Filter by category"
        disabled={disabled}
        defaultValue={selectedCategoryId}
      >
        <option value="">{ALL_CATEGORIES_LABEL}</option>
        {(categories ?? []).map((category) => (
          <option key={category.id} value={category.id}>
            {categoryDisplayName(category)}
          </option>
        ))}
        <option value={CATEGORY_ACTION_CREATE} disabled={disabled}>
          + Add category
        </option>
        <option value={CATEGORY_ACTION_DELETE} disabled={deleteDisabled}>
          Delete category
        </option>
      </select>
      {categoryError ? (
        <p className="todo-task-category-error" role="alert">
          {categoryError}
        </p>
      ) : null}
    </div>
  );
}

function measureOptionLabelWidth(text: unknown, font: string) {
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
  return label.length * 8;
}

export function syncCategoryFilterWidth(root: { querySelector?: (sel: string) => Element | null } | null) {
  const select = root?.querySelector?.('.todo-task-category-filter');
  if (!(select instanceof HTMLSelectElement)) return;
  const font = getComputedStyle(select).font || '13px sans-serif';
  let maxPx = 0;
  for (const opt of select.options) {
    maxPx = Math.max(maxPx, measureOptionLabelWidth(opt.textContent, font));
  }
  const chromePx = 42;
  select.style.width = `${Math.ceil(maxPx + chromePx)}px`;
}

export function PageHeader({
  disabled,
  activeOnly = true,
  categories = [],
  selectedCategoryId = '',
  categoryError = '',
}: {
  disabled?: boolean;
  activeOnly?: boolean;
  categories?: TodoCategory[];
  selectedCategoryId?: string;
  categoryError?: string;
}) {
  return (
    <header className="todo-tasks-page-header">
      <h1 className="todo-tasks-page-title">Todos</h1>
      <div className="todo-tasks-page-header-actions">
        <CategoryControls
          categories={categories}
          selectedCategoryId={selectedCategoryId}
          categoryError={categoryError}
          disabled={Boolean(disabled)}
        />
        <label className="todo-task-active-only">
          <span className="todo-task-active-only-label">{ACTIVE_ONLY_LABEL}</span>
          <button
            type="button"
            className="todo-task-active-only-switch"
            role="switch"
            data-action="toggle-active-only"
            aria-checked={activeOnly ? 'true' : 'false'}
            aria-label={ACTIVE_ONLY_LABEL}
            disabled={disabled}
          />
        </label>
        <button type="button" className="md-header-btn primary" data-action="create-master" disabled={disabled}>
          + New todo
        </button>
      </div>
    </header>
  );
}

export function renderPageHeader(
  disabled: boolean,
  activeOnly = true,
  categories: TodoCategory[] = [],
  selectedCategoryId = '',
  categoryError = '',
) {
  return renderToHtml(
    <PageHeader
      disabled={disabled}
      activeOnly={activeOnly}
      categories={categories}
      selectedCategoryId={selectedCategoryId}
      categoryError={categoryError}
    />,
  );
}

function MasterList({
  masters,
  selectedMasterId,
  disabled,
}: {
  masters: TodoMaster[];
  selectedMasterId: string;
  disabled: boolean;
}) {
  return (
    <ul className="todo-task-master-list" role="list">
      {sortMasters(masters).map((master: TodoMaster) => {
        const selected =
          master.master_task_id === selectedMasterId ? ' todo-task-master-item--selected' : '';
        const statusMod = ` todo-task-master-item--${masterStatusClass(master.status)}`;
        const subCount = master.sub_tasks?.length ?? 0;
        const meta = `${subCount} sub-tasks · ${formatRelativeTime(master.created_at) || 'Unknown time'}`;
        return (
          <li key={master.master_task_id}>
            <button
              type="button"
              className={`todo-task-master-item${selected}${statusMod}`}
              data-master-id={master.master_task_id}
              disabled={disabled}
            >
              <span className="todo-task-master-title">{master.title}</span>
              <span className="todo-task-master-meta">{meta}</span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}

function MasterEmpty({ disabled }: { disabled: boolean }) {
  return (
    <div className="todo-task-empty todo-task-empty--sidebar">
      <p className="todo-task-empty-title">No todos yet</p>
      <p className="todo-task-empty-detail">Create your first todo to manage sub-tasks</p>
      <button type="button" className="md-header-btn primary" data-action="create-master" disabled={disabled}>
        New todo
      </button>
    </div>
  );
}

function ActiveOnlyEmpty() {
  return (
    <div className="todo-task-empty todo-task-empty--sidebar">
      <p className="todo-task-empty-title">{ACTIVE_ONLY_EMPTY_TITLE}</p>
      <p className="todo-task-empty-detail">{ACTIVE_ONLY_EMPTY_DETAIL}</p>
    </div>
  );
}

function CategoryFilterEmpty() {
  return (
    <div className="todo-task-empty todo-task-empty--sidebar">
      <p className="todo-task-empty-title">{CATEGORY_FILTER_EMPTY_TITLE}</p>
      <p className="todo-task-empty-detail">{CATEGORY_FILTER_EMPTY_DETAIL}</p>
    </div>
  );
}

export function MasterPane({
  masters,
  selectedMasterId,
  disabled,
  activeOnly = true,
  categoryId = '',
}: {
  masters: TodoMaster[];
  selectedMasterId: string;
  disabled: boolean;
  activeOnly?: boolean;
  categoryId?: string;
}) {
  const visible = filterMastersForView(masters, activeOnly, categoryId);
  if (!masters.length) {
    return <MasterEmpty disabled={disabled} />;
  }
  if (!visible.length) {
    if (categoryId && filterMastersForView(masters, activeOnly, '').length) {
      return <CategoryFilterEmpty />;
    }
    return <ActiveOnlyEmpty />;
  }
  return <MasterList masters={visible} selectedMasterId={selectedMasterId} disabled={disabled} />;
}

export function renderMasterPane(
  masters: TodoMaster[],
  selectedMasterId: string,
  disabled: boolean,
  activeOnly = true,
  categoryId = '',
) {
  return renderToHtml(
    <MasterPane
      masters={masters}
      selectedMasterId={selectedMasterId}
      disabled={disabled}
      activeOnly={activeOnly}
      categoryId={categoryId}
    />,
  );
}
