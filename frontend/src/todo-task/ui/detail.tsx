import { renderToHtml } from '../../island.ts';
import { DEFAULT_PLAN_CATEGORY_ID } from '../state/host.ts';
import {
  categoryDisplayName,
  copySubIdPair,
  formatRelativeTime,
  formatTodoTaskStatus,
  masterCategoryId,
  masterStatusClass,
} from '../state/format.ts';
import { PlanMdSection } from './plan-md.tsx';
import { AttachmentsSection } from './attachments.tsx';
import { CommentsSection } from './comments.tsx';

const MIGRATION_WARNING_MSG = 'Todo migration incomplete; some data may be missing';

type SubTask = {
  sub_task_id: string;
  title?: string;
  status?: string;
  content?: string;
  linked_archive_ids?: string[];
};

type Master = {
  master_task_id: string;
  title?: string;
  status?: string;
  created_at?: number | string;
  sub_tasks?: SubTask[];
  migration_error?: unknown;
};

type DetailUi = {
  disabled?: boolean;
  subStatus?: Record<string, string>;
  subTitleDrafts?: Record<string, string>;
  subActionErrors?: Record<string, string>;
  subTitleErrors?: Record<string, string>;
  expandedSubContent?: Record<string, boolean>;
  subContentDrafts?: Record<string, string>;
  masterTitleDraft?: string;
  masterTitleError?: string;
  categories?: { id: string; name?: string; is_default?: boolean }[];
  refreshWarning?: string;
};

function LinkedArchives({ ids }: { ids?: string[] }) {
  if (!ids?.length) return null;
  return <div className="todo-task-sub-archives">Linked archives:{ids.join(', ')}</div>;
}

function SubStatusSelect({ sub, disabled }: { sub: SubTask; disabled?: boolean }) {
  return (
    <select
      className={`todo-task-status-select todo-task-sub-status-select todo-task-sub-status-select--${sub.status}`}
      data-action="change-sub-status"
      data-sub-id={sub.sub_task_id}
      aria-label="Sub-task status"
      disabled={disabled || sub.status !== 'incomplete'}
      defaultValue={sub.status}
    >
      {['incomplete', 'complete', 'abandoned'].map((status) => (
        <option key={status} value={status}>
          {formatTodoTaskStatus(status)}
        </option>
      ))}
    </select>
  );
}

export function SubRow({
  master,
  sub,
  selectedSubId,
  ui,
}: {
  master: Master;
  sub: SubTask;
  selectedSubId: string;
  ui: DetailUi;
}) {
  const effectiveStatus = ui.subStatus?.[sub.sub_task_id] ?? sub.status;
  const subForRender = { ...sub, status: effectiveStatus };
  const selected = sub.sub_task_id === selectedSubId ? ' todo-task-sub--selected' : '';
  const title = ui.subTitleDrafts?.[sub.sub_task_id] ?? (sub.title || sub.sub_task_id);
  const copyText = copySubIdPair(master.master_task_id, sub.sub_task_id);
  const subError = ui.subActionErrors?.[sub.sub_task_id] ?? '';
  const titleError = ui.subTitleErrors?.[sub.sub_task_id] ?? '';
  const expanded = Boolean(ui.expandedSubContent?.[sub.sub_task_id]);
  const contentValue =
    ui.subContentDrafts?.[sub.sub_task_id] ?? (typeof sub.content === 'string' ? sub.content : '');
  return (
    <article data-sub-id={sub.sub_task_id} className={`todo-task-sub${selected}`}>
      <header className="todo-task-sub-header">
        <input
          type="text"
          className="todo-task-sub-title-input"
          data-action="edit-sub-title"
          data-sub-id={sub.sub_task_id}
          defaultValue={title}
          aria-label="Sub-task title"
          disabled={ui.disabled}
        />
        <div className="todo-task-sub-header-actions">
          <button
            type="button"
            className="md-header-btn todo-task-sub-content-toggle"
            data-action="toggle-sub-content"
            data-sub-id={sub.sub_task_id}
            aria-expanded={expanded ? 'true' : 'false'}
            aria-label="Edit content"
            disabled={ui.disabled}
          >
            Content
          </button>
          <SubStatusSelect sub={subForRender} disabled={ui.disabled} />
          <div className="todo-task-sub-menu">
            <button
              type="button"
              className="todo-task-sub-menu-btn"
              data-action="toggle-sub-menu"
              aria-label="More actions"
              disabled={ui.disabled}
            >
              ⋯
            </button>
            <div className="todo-task-sub-menu-panel" hidden>
              <button type="button" data-action="copy-sub-id" data-copy-text={copyText}>
                Copy ID
              </button>
              <button
                type="button"
                data-action="delete-sub"
                data-sub-id={sub.sub_task_id}
                data-sub-title={title}
              >
                Delete
              </button>
            </div>
          </div>
        </div>
      </header>
      {titleError ? (
        <p className="todo-task-sub-title-error" role="alert">
          {titleError}
        </p>
      ) : null}
      {subError ? (
        <p className="todo-task-sub-action-error" role="alert">
          {subError}
        </p>
      ) : null}
      {expanded ? (
        <textarea
          className="todo-task-sub-content-editor"
          data-action="edit-sub-content"
          data-sub-id={sub.sub_task_id}
          aria-label="Sub-task content"
          disabled={ui.disabled}
          defaultValue={contentValue}
        />
      ) : null}
      <LinkedArchives ids={sub.linked_archive_ids} />
    </article>
  );
}

function MasterStatusSelect({ status, disabled }: { status: string; disabled?: boolean }) {
  const wire = masterStatusClass(status);
  return (
    <select
      className={`todo-task-status-select todo-task-master-status-select todo-task-master-status-select--${wire}`}
      data-action="change-master-status"
      aria-label="Todo status"
      disabled={disabled}
      defaultValue={wire}
    >
      {['incomplete', 'complete', 'abandoned'].map((value) => (
        <option key={value} value={value}>
          {formatTodoTaskStatus(value)}
        </option>
      ))}
    </select>
  );
}

function MasterCategorySelect({
  master,
  categories,
  disabled,
}: {
  master: Master;
  categories?: { id: string; name?: string; is_default?: boolean }[];
  disabled?: boolean;
}) {
  const current = masterCategoryId(master);
  const list = categories?.length
    ? categories
    : [{ id: DEFAULT_PLAN_CATEGORY_ID, name: 'Uncategorized', is_default: true }];
  return (
    <select
      className="todo-task-category-select"
      data-action="change-master-category"
      aria-label="Todo category"
      disabled={disabled}
      defaultValue={current}
    >
      {list.map((category) => (
        <option key={category.id} value={category.id}>
          {categoryDisplayName(category)}
        </option>
      ))}
    </select>
  );
}

export function DetailTitle({ master, ui = {} }: { master: Master; ui?: DetailUi }) {
  const title = ui.masterTitleDraft ?? master.title ?? '';
  const status = masterStatusClass(master.status);
  return (
    <>
      <div className="todo-task-detail-title-row">
        <input
          type="text"
          className={`todo-task-detail-title todo-task-detail-title--${status}`}
          data-action="edit-master-title"
          defaultValue={title}
          aria-label="Todo title"
          disabled={ui.disabled}
        />
        <MasterCategorySelect master={master} categories={ui.categories} disabled={ui.disabled} />
        <MasterStatusSelect status={status} disabled={ui.disabled} />
      </div>
      {ui.masterTitleError ? (
        <p className="todo-task-detail-title-error">{ui.masterTitleError}</p>
      ) : null}
    </>
  );
}

export function renderDetailTitle(master: Master, ui: DetailUi = {}) {
  return renderToHtml(<DetailTitle master={master} ui={ui} />);
}

export function renderSubRow(master: Master, sub: SubTask, selectedSubId: string, ui: DetailUi) {
  return renderToHtml(<SubRow master={master} sub={sub} selectedSubId={selectedSubId} ui={ui} />);
}

export function renderSubDetail(master: Master, selectedSubId: string) {
  const subs = master.sub_tasks ?? [];
  const ui = { disabled: false, subStatus: {}, subActionErrors: {} };
  return renderToHtml(
    <div className="todo-task-detail-body">
      <DetailTitle master={master} ui={ui} />
      <div className="todo-task-sub-list">
        {subs.map((sub) => (
          <SubRow key={sub.sub_task_id} master={master} sub={sub} selectedSubId={selectedSubId} ui={ui} />
        ))}
      </div>
    </div>,
  );
}

function DetailMeta({ master }: { master: Master }) {
  const subCount = master.sub_tasks?.length ?? 0;
  const created = formatRelativeTime(master.created_at) || 'Unknown time';
  return (
    <p className="todo-task-detail-meta">
      {subCount} sub-tasks · created {created}
    </p>
  );
}

function DetailToolbar({ masterTaskId, disabled }: { masterTaskId: string; disabled?: boolean }) {
  return (
    <div className="todo-task-detail-toolbar">
      <button
        type="button"
        className="md-header-btn"
        data-action="add-sub"
        data-master-id={masterTaskId}
        disabled={disabled}
      >
        Add sub-task
      </button>
      <button type="button" className="md-header-btn todo-task-btn-danger" data-action="delete-master" disabled={disabled}>
        Delete todo
      </button>
    </div>
  );
}

function SubEmpty({ disabled }: { disabled?: boolean }) {
  return (
    <div className="todo-task-empty todo-task-empty--detail">
      <p className="todo-task-empty-title">No sub-tasks yet</p>
      <button type="button" className="md-header-btn primary" data-action="add-sub" disabled={disabled}>
        Add sub-task
      </button>
    </div>
  );
}

export function SubDetailPane({
  master,
  selectedSubId,
  ui,
}: {
  master: Master;
  selectedSubId: string;
  ui: DetailUi;
}) {
  const subs = master.sub_tasks ?? [];
  return (
    <div className="todo-task-detail-body">
      <div className="todo-task-detail-header">
        <div>
          <DetailTitle master={master} ui={ui} />
          <DetailMeta master={master} />
        </div>
      </div>
      {master.migration_error ? (
        <div className="todo-task-migration-warning todo-task-split-state" role="status">
          <p className="todo-task-split-state-detail">{MIGRATION_WARNING_MSG}</p>
        </div>
      ) : null}
      <PlanMdSection master={master} ui={ui} />
      <AttachmentsSection ui={ui} />
      {ui.refreshWarning ? (
        <div className="todo-task-refresh-warning todo-task-split-state" role="status">
          <p className="todo-task-split-state-detail">{ui.refreshWarning}</p>
          <button
            type="button"
            className="md-header-btn todo-task-refresh-retry"
            disabled={ui.disabled}
            data-action="retry-refresh"
          >
            Retry refresh
          </button>
        </div>
      ) : null}
      <DetailToolbar masterTaskId={master.master_task_id} disabled={ui.disabled} />
      <div className="todo-task-sub-list">
        {subs.length ? (
          subs.map((sub) => (
            <SubRow key={sub.sub_task_id} master={master} sub={sub} selectedSubId={selectedSubId} ui={ui} />
          ))
        ) : (
          <SubEmpty disabled={ui.disabled} />
        )}
      </div>
      <CommentsSection ui={ui} />
    </div>
  );
}

export function renderSubDetailPane(master: Master, selectedSubId: string, ui: DetailUi) {
  return renderToHtml(<SubDetailPane master={master} selectedSubId={selectedSubId} ui={ui} />);
}
