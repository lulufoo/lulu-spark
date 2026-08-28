import type { ReactNode } from 'react';
import { renderToHtml } from '../../island.ts';
import { PageHeader } from './list.tsx';

export const UNAVAILABLE_MSG = 'List temporarily unavailable. Please try again later.';
export const REFRESH_WARNING_MSG = 'Saved, but list refresh failed — retry';

export function DetailEmpty() {
  return (
    <div className="todo-task-split-detail-empty todo-task-empty">
      <p className="todo-task-empty-title">Select a todo on the left</p>
      <p className="todo-task-empty-detail">Or create a todo from the top right</p>
    </div>
  );
}

export function DeadLink() {
  return (
    <div className="todo-task-split-dead-link todo-task-split-state">
      <p className="todo-task-split-state-title">Task not found</p>
      <p className="todo-task-split-state-detail">Link may be stale — pick again from the list</p>
    </div>
  );
}

export function ErrorEmpty({ message = UNAVAILABLE_MSG }: { message?: string }) {
  return (
    <div className="todo-task-split-error todo-task-split-state todo-task-split-state--error">
      <p className="todo-task-split-state-title">Temporarily unavailable</p>
      <p className="todo-task-split-state-detail">{message}</p>
    </div>
  );
}

export function renderDetailEmpty() {
  return renderToHtml(<DetailEmpty />);
}

export function renderDeadLink() {
  return renderToHtml(<DeadLink />);
}

export function renderErrorEmpty(message = UNAVAILABLE_MSG) {
  return renderToHtml(<ErrorEmpty message={message} />);
}

export function PageShell({
  master,
  detail,
  disabled = false,
  activeOnly = true,
  categories = [],
  filterCategoryId = '',
  categoryError = '',
}: {
  master: ReactNode;
  detail: ReactNode;
  disabled?: boolean;
  activeOnly?: boolean;
  categories?: { id: string; name?: string; is_default?: boolean }[];
  filterCategoryId?: string;
  categoryError?: string;
}) {
  return (
    <div className="todo-tasks-page">
      <PageHeader
        disabled={disabled}
        activeOnly={activeOnly}
        categories={categories}
        selectedCategoryId={filterCategoryId}
        categoryError={categoryError}
      />
      <div className="todo-task-split">
        <aside className="todo-task-split-master" aria-label="Todos list">
          {master}
        </aside>
        <section className="todo-task-split-detail" aria-label="Task details">
          {detail}
        </section>
      </div>
    </div>
  );
}

export function renderPageShell({
  masterHtml,
  detailHtml,
  disabled = false,
  activeOnly = true,
  categories = [],
  filterCategoryId = '',
  categoryError = '',
}: {
  masterHtml: string;
  detailHtml: string;
  disabled?: boolean;
  activeOnly?: boolean;
  categories?: { id: string; name?: string; is_default?: boolean }[];
  filterCategoryId?: string;
  categoryError?: string;
}) {
  return renderToHtml(
    <PageShell
      disabled={disabled}
      activeOnly={activeOnly}
      categories={categories}
      filterCategoryId={filterCategoryId}
      categoryError={categoryError}
      master={<div dangerouslySetInnerHTML={{ __html: masterHtml }} />}
      detail={<div dangerouslySetInnerHTML={{ __html: detailHtml }} />}
    />,
  );
}
