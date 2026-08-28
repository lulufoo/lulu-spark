import type { ReactNode } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { flushSync } from 'react-dom';

export type TodoTaskDialogType =
  | 'create-master'
  | 'create-category'
  | 'add-sub'
  | 'delete-master'
  | 'delete-sub';

let lastTrigger: HTMLElement | null = null;
let submitHandler: ((values: Record<string, unknown>) => Promise<void>) | null = null;
let keydownHandler: ((event: KeyboardEvent) => void) | null = null;
let bodyRoot: Root | null = null;
let bodyHost: Element | null = null;

function getDialogEl() {
  return document.getElementById('todo-task-dialog');
}

function getBodyEl() {
  return document.getElementById('todo-task-dialog-body');
}

function getErrorEl() {
  return document.getElementById('todo-task-dialog-error');
}

function getPrimaryBtn() {
  return document.getElementById('todo-task-dialog-primary') as HTMLButtonElement | null;
}

function getCancelBtn() {
  return document.getElementById('todo-task-dialog-cancel') as HTMLButtonElement | null;
}

function getTitleEl() {
  return document.getElementById('todo-task-dialog-title');
}

function setDialogError(message: string) {
  const el = getErrorEl();
  if (!el) return;
  if (!message) {
    el.hidden = true;
    el.textContent = '';
    return;
  }
  el.hidden = false;
  el.textContent = message;
}

function setSubmitLoading(loading: boolean) {
  const primary = getPrimaryBtn();
  const cancel = getCancelBtn();
  if (!primary || !cancel) return;
  primary.disabled = loading;
  cancel.disabled = loading;
  if (loading) {
    primary.dataset.originalLabel = primary.textContent ?? '';
    primary.textContent = 'Saving…';
  } else if (primary.dataset.originalLabel) {
    primary.textContent = primary.dataset.originalLabel;
    delete primary.dataset.originalLabel;
  }
}

function collectSubTitleRows() {
  const body = getBodyEl();
  if (!body) return [];
  return [...body.querySelectorAll('[data-sub-row-input]')]
    .map((rowInput) => (rowInput as HTMLInputElement).value.trim())
    .filter(Boolean);
}

function SubTitleRows({ rows }: { rows: string[] }) {
  return (
    <>
      <div className="todo-task-dialog-sub-rows" data-sub-rows="">
        {rows.map((value, index) => (
          <div className="todo-task-dialog-sub-row" key={`${index}-${value}`}>
            <input
              type="text"
              className="todo-task-dialog-field"
              data-sub-row-input=""
              placeholder={`Sub-task ${index + 1}`}
              defaultValue={value}
            />
            <button
              type="button"
              className="todo-task-dialog-row-remove"
              data-action="remove-sub-row"
              aria-label="Remove"
              disabled={rows.length <= 1}
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
    </>
  );
}

function paintBody(node: ReactNode) {
  const body = getBodyEl();
  if (!body) return;
  if (bodyRoot && bodyHost !== body) {
    flushSync(() => {
      bodyRoot?.unmount();
    });
    bodyRoot = null;
  }
  if (!bodyRoot) {
    bodyRoot = createRoot(body);
    bodyHost = body;
  }
  flushSync(() => {
    bodyRoot?.render(node);
  });
}

function renderDialogBody(type: TodoTaskDialogType, payload: Record<string, unknown>) {
  const title = getTitleEl();
  const primary = getPrimaryBtn();
  if (!getBodyEl() || !title || !primary) return;

  setDialogError('');
  primary.classList.remove('danger');

  if (type === 'create-master') {
    title.textContent = 'New todo';
    primary.textContent = 'Create todo';
    paintBody(
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
          />
        </label>
        <div className="todo-task-dialog-section">
          <span className="todo-task-dialog-label-text">Initial sub-tasks (optional)</span>
          <SubTitleRows rows={['']} />
        </div>
      </>,
    );
    return;
  }

  if (type === 'create-category') {
    title.textContent = 'New category';
    primary.textContent = 'Create category';
    paintBody(
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
          />
        </label>
        <p className="todo-task-dialog-hint">Use a short, unique name. You can delete empty categories later.</p>
      </>,
    );
    return;
  }

  if (type === 'add-sub') {
    title.textContent = 'Add sub-task';
    primary.textContent = 'Add';
    paintBody(
      <>
        <p className="todo-task-dialog-readonly">
          Parent todo:{String(payload.masterTitle ?? '')}
        </p>
        <label className="todo-task-dialog-label">
          <span className="todo-task-dialog-label-text">
            Sub-task name <span className="todo-task-dialog-required">*</span>
          </span>
          <input type="text" className="todo-task-dialog-field" data-field="title" placeholder="Required" />
        </label>
      </>,
    );
    return;
  }

  if (type === 'delete-master') {
    title.textContent = 'Delete todo?';
    primary.textContent = 'Delete todo';
    primary.classList.add('danger');
    const subCount = Number(payload.subCount ?? 0);
    paintBody(
      <p className="todo-task-dialog-message">
        Permanently delete “{String(payload.masterTitle ?? '')}” and its {subCount} sub-tasks. This cannot
        be undone.
      </p>,
    );
    return;
  }

  if (type === 'delete-sub') {
    title.textContent = 'Delete sub-task?';
    primary.textContent = 'Delete';
    primary.classList.add('danger');
    paintBody(
      <p className="todo-task-dialog-message">Delete “{String(payload.subTitle ?? '')}”.</p>,
    );
  }
}

function focusFirstField() {
  const body = getBodyEl();
  const first = body?.querySelector('input.todo-task-dialog-field, [data-sub-row-input]');
  if (first instanceof HTMLElement) {
    first.focus();
  }
}

function restoreFocus() {
  if (lastTrigger instanceof HTMLElement && document.contains(lastTrigger)) {
    lastTrigger.focus();
  }
  lastTrigger = null;
}

export function closeTodoTaskDialog() {
  const dialog = getDialogEl();
  if (!dialog) return;
  dialog.classList.remove('open');
  setSubmitLoading(false);
  setDialogError('');
  submitHandler = null;
  if (keydownHandler) {
    document.removeEventListener('keydown', keydownHandler);
    keydownHandler = null;
  }
  restoreFocus();
  document.dispatchEvent(new CustomEvent('todo-task-dialog-close'));
}

export function openTodoTaskDialog({
  type,
  payload = {},
  onSubmit,
  triggerEl = null,
}: {
  type: TodoTaskDialogType;
  payload?: Record<string, unknown>;
  onSubmit: (values: Record<string, unknown>) => Promise<void>;
  triggerEl?: HTMLElement | null;
}) {
  wireTodoTaskDialog();
  const dialog = getDialogEl();
  if (!dialog) return;

  lastTrigger = triggerEl;
  submitHandler = onSubmit;
  dialog.dataset.dialogType = type;
  renderDialogBody(type, payload);
  dialog.classList.add('open');
  focusFirstField();

  keydownHandler = (event) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      closeTodoTaskDialog();
      return;
    }
    if (event.key === 'Enter' && !event.shiftKey) {
      const target = event.target;
      if (target instanceof HTMLTextAreaElement) return;
      if (target instanceof HTMLButtonElement && target.dataset.action === 'add-sub-row') return;
      event.preventDefault();
      void handleSubmit(type);
    }
  };
  document.addEventListener('keydown', keydownHandler);
}

async function handleSubmit(type: TodoTaskDialogType) {
  if (!submitHandler) return;
  const body = getBodyEl();
  if (!body) return;

  if (type === 'create-master') {
    const title = (body.querySelector('[data-field="title"]') as HTMLInputElement | null)?.value?.trim() ?? '';
    if (!title) {
      setDialogError('Please enter a todo name');
      return;
    }
    const subTitles = collectSubTitleRows();
    setSubmitLoading(true);
    setDialogError('');
    try {
      await submitHandler({ title, subTitles: subTitles.length ? subTitles : undefined });
      closeTodoTaskDialog();
    } catch (err) {
      setDialogError((err as Error)?.message || 'Operation failed');
      setSubmitLoading(false);
    }
    return;
  }

  if (type === 'create-category') {
    const name = (body.querySelector('[data-field="name"]') as HTMLInputElement | null)?.value?.trim() ?? '';
    if (!name) {
      setDialogError('Please enter a category name');
      return;
    }
    setSubmitLoading(true);
    setDialogError('');
    try {
      await submitHandler({ name });
      closeTodoTaskDialog();
    } catch (err) {
      setDialogError((err as Error)?.message || 'Failed to create category');
      setSubmitLoading(false);
    }
    return;
  }

  if (type === 'add-sub') {
    const title = (body.querySelector('[data-field="title"]') as HTMLInputElement | null)?.value?.trim() ?? '';
    if (!title) {
      setDialogError('Please enter a sub-task name');
      return;
    }
    setSubmitLoading(true);
    setDialogError('');
    try {
      await submitHandler({ title });
      closeTodoTaskDialog();
    } catch (err) {
      setDialogError((err as Error)?.message || 'Operation failed');
      setSubmitLoading(false);
    }
    return;
  }

  setSubmitLoading(true);
  setDialogError('');
  try {
    await submitHandler({});
    closeTodoTaskDialog();
  } catch (err) {
    setDialogError((err as Error)?.message || 'Operation failed');
    setSubmitLoading(false);
  }
}

function paintSubRows(rows: string[]) {
  const titleInput = getBodyEl()?.querySelector('[data-field="title"]') as HTMLInputElement | null;
  const titleValue = titleInput?.value ?? '';
  paintBody(
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
          defaultValue={titleValue}
        />
      </label>
      <div className="todo-task-dialog-section">
        <span className="todo-task-dialog-label-text">Initial sub-tasks (optional)</span>
        <SubTitleRows rows={rows.length ? rows : ['']} />
      </div>
    </>,
  );
}

function wireTodoTaskDialog() {
  const dialog = getDialogEl();
  if (!dialog || dialog.dataset.wired === '1') return;
  dialog.dataset.wired = '1';

  dialog.addEventListener('click', (event) => {
    if (event.target === dialog) {
      closeTodoTaskDialog();
    }
  });

  getCancelBtn()?.addEventListener('click', () => closeTodoTaskDialog());
  getPrimaryBtn()?.addEventListener('click', () => {
    const type = dialog.dataset.dialogType as TodoTaskDialogType | undefined;
    if (type) void handleSubmit(type);
  });

  dialog.addEventListener('click', (event) => {
    const actionEl = (event.target as HTMLElement | null)?.closest('[data-action]') as HTMLElement | null;
    const action = actionEl?.dataset.action;
    if (action === 'add-sub-row') {
      event.preventDefault();
      if (!dialog.querySelector('[data-sub-rows]')) return;
      const rows = collectSubTitleRows();
      rows.push('');
      paintSubRows(rows);
      const inputs = dialog.querySelectorAll('[data-sub-row-input]');
      const last = inputs[inputs.length - 1];
      if (last instanceof HTMLElement) last.focus();
      return;
    }
    if (action === 'remove-sub-row') {
      event.preventDefault();
      const row = actionEl?.closest('.todo-task-dialog-sub-row');
      const rowInput = row?.querySelector('[data-sub-row-input]');
      if (!(rowInput instanceof HTMLInputElement)) return;
      const allInputs = [...dialog.querySelectorAll('[data-sub-row-input]')];
      const rows = allInputs
        .filter((node) => node !== rowInput)
        .map((node) => (node as HTMLInputElement).value.trim());
      paintSubRows(rows.length ? rows : ['']);
    }
  });
}

wireTodoTaskDialog();
