import type { MouseEvent } from 'react';
import { useSyncExternalStore } from 'react';
import { closeTodoTaskDialog, handleSubmit } from '../commands/dialog.ts';
import { openStore, patchView, viewStore, type DialogView } from '../state/dialog.ts';

export { closeTodoTaskDialog, openTodoTaskDialog } from '../commands/dialog.ts';
export type { TodoTaskDialogType } from '../state/dialog.ts';

function DialogBody({ view }: { view: DialogView }) {
  if (view.type === 'create-master') {
    return (
      <>
        <label className="todo-task-dialog-label">
          <span className="todo-task-dialog-label-text">
            Todo name <span className="todo-task-dialog-required">*</span>
          </span>
          <input
            type="text"
            className="todo-task-dialog-field"
            data-field="title"
            placeholder="e.g. FM-4 UI polish"
            value={view.titleField}
            onChange={(e) => patchView({ titleField: e.target.value })}
          />
        </label>
        <div className="todo-task-dialog-section">
          <span className="todo-task-dialog-label-text">Initial sub-tasks (optional)</span>
          <div className="todo-task-dialog-sub-rows" data-sub-rows="">
            {view.subRows.map((value, index) => (
              <div className="todo-task-dialog-sub-row" key={index}>
                <input
                  type="text"
                  className="todo-task-dialog-field"
                  data-sub-row-input=""
                  placeholder={`Sub-task ${index + 1}`}
                  value={value}
                  onChange={(e) => {
                    const next = view.subRows.slice();
                    next[index] = e.target.value;
                    patchView({ subRows: next });
                  }}
                />
                <button
                  type="button"
                  className="todo-task-dialog-row-remove"
                  data-action="remove-sub-row"
                  aria-label="Remove"
                  disabled={view.subRows.length <= 1}
                >
                  ×
                </button>
              </div>
            ))}
          </div>
          <button type="button" className="todo-task-dialog-add-row" data-action="add-sub-row">
            + Add sub-task row
          </button>
          <p className="todo-task-dialog-hint">If you skip sub-tasks, the todo will have none</p>
        </div>
      </>
    );
  }
  if (view.type === 'create-category') {
    return (
      <>
        <p className="todo-task-dialog-lead">
          Organize todos under a shared category. The name is available in the Todos filter and MCP tools.
        </p>
        <label className="todo-task-dialog-label">
          <span className="todo-task-dialog-label-text">
            Category name <span className="todo-task-dialog-required">*</span>
          </span>
          <input
            type="text"
            className="todo-task-dialog-field"
            data-field="name"
            maxLength={64}
            autoComplete="off"
            spellCheck={false}
            placeholder="e.g. Product launch"
            value={view.nameField}
            onChange={(e) => patchView({ nameField: e.target.value })}
          />
        </label>
        <p className="todo-task-dialog-hint">Use a short, unique name. You can delete empty categories later.</p>
      </>
    );
  }
  if (view.type === 'add-sub') {
    return (
      <>
        <p className="todo-task-dialog-readonly">
          Parent todo:{String(view.payload.masterTitle ?? '')}
        </p>
        <label className="todo-task-dialog-label">
          <span className="todo-task-dialog-label-text">
            Sub-task name <span className="todo-task-dialog-required">*</span>
          </span>
          <input
            type="text"
            className="todo-task-dialog-field"
            data-field="title"
            placeholder="Required"
            value={view.subTitleField}
            onChange={(e) => patchView({ subTitleField: e.target.value })}
          />
        </label>
      </>
    );
  }
  if (view.type === 'delete-master') {
    const subCount = Number(view.payload.subCount ?? 0);
    return (
      <p className="todo-task-dialog-message">
        Permanently delete “{String(view.payload.masterTitle ?? '')}” and its {subCount} sub-tasks. This cannot
        be undone.
      </p>
    );
  }
  if (view.type === 'delete-sub') {
    return <p className="todo-task-dialog-message">Delete “{String(view.payload.subTitle ?? '')}”.</p>;
  }
  return null;
}

function onDialogClick(event: MouseEvent<HTMLDivElement>) {
  const dialog = event.currentTarget;
  if (event.target === dialog) {
    closeTodoTaskDialog();
    return;
  }

  const actionEl = (event.target as HTMLElement | null)?.closest('[data-action]') as HTMLElement | null;
  const action = actionEl?.dataset.action;
  const view = viewStore.getSnapshot();
  if (action === 'add-sub-row') {
    event.preventDefault();
    patchView({ subRows: [...view.subRows, ''] });
    return;
  }
  if (action === 'remove-sub-row') {
    event.preventDefault();
    const row = actionEl?.closest('.todo-task-dialog-sub-row');
    if (!row) return;
    const index = [...dialog.querySelectorAll('.todo-task-dialog-sub-row')].indexOf(row);
    if (index < 0) return;
    const next = view.subRows.filter((_, i) => i !== index);
    patchView({ subRows: next.length ? next : [''] });
  }
}

export function TodoTaskDialog() {
  const open = useSyncExternalStore(openStore.subscribe, openStore.getSnapshot);
  const view = useSyncExternalStore(viewStore.subscribe, viewStore.getSnapshot);

  return (
    <div
      id="todo-task-dialog"
      className={open ? 'open' : undefined}
      data-dialog-type={view.type ?? undefined}
      onClick={onDialogClick}
    >
      <div id="todo-task-dialog-box">
        <div id="todo-task-dialog-header">
          <h3 id="todo-task-dialog-title">{view.title}</h3>
        </div>
        <div id="todo-task-dialog-body">
          <DialogBody view={view} />
        </div>
        <p id="todo-task-dialog-error" hidden={!view.error}>
          {view.error}
        </p>
        <div id="todo-task-dialog-actions">
          <button
            type="button"
            id="todo-task-dialog-cancel"
            className="md-header-btn"
            disabled={view.loading}
            onClick={() => closeTodoTaskDialog()}
          >
            Cancel
          </button>
          <button
            type="button"
            id="todo-task-dialog-primary"
            className={view.danger ? 'md-header-btn primary danger' : 'md-header-btn primary'}
            disabled={view.loading}
            onClick={() => {
              if (view.type) void handleSubmit(view.type);
            }}
          >
            {view.loading ? 'Saving…' : view.primaryLabel || 'OK'}
          </button>
        </div>
      </div>
    </div>
  );
}
