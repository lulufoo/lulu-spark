import { flushSync } from 'react-dom';
import {
  emptyDialogView,
  openStore,
  patchView,
  viewStore,
  type TodoTaskDialogType,
  type DialogView,
} from '../state/dialog.ts';

export type { TodoTaskDialogType, DialogView };

let lastTrigger: HTMLElement | null = null;
let submitHandler: ((values: Record<string, unknown>) => Promise<void>) | null = null;
let keydownHandler: ((event: KeyboardEvent) => void) | null = null;

function dualWriteOpen(open: boolean) {
  const dialog = document.getElementById('todo-task-dialog');
  if (!dialog) return;
  if (open) dialog.classList.add('open');
  else dialog.classList.remove('open');
}

function viewForType(type: TodoTaskDialogType, payload: Record<string, unknown>): DialogView {
  const base = { ...emptyDialogView(), type, payload };
  if (type === 'create-master') {
    return { ...base, title: 'New todo', primaryLabel: 'Create todo', subRows: [''] };
  }
  if (type === 'create-category') {
    return { ...base, title: 'New category', primaryLabel: 'Create category' };
  }
  if (type === 'add-sub') {
    return { ...base, title: 'Add sub-task', primaryLabel: 'Add' };
  }
  if (type === 'delete-master') {
    return { ...base, title: 'Delete todo?', primaryLabel: 'Delete todo', danger: true };
  }
  return { ...base, title: 'Delete sub-task?', primaryLabel: 'Delete', danger: true };
}

function restoreFocus() {
  if (lastTrigger instanceof HTMLElement && document.contains(lastTrigger)) {
    lastTrigger.focus();
  }
  lastTrigger = null;
}

export function closeTodoTaskDialog() {
  openStore.set(false);
  viewStore.set(emptyDialogView());
  dualWriteOpen(false);
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
  lastTrigger = triggerEl;
  submitHandler = onSubmit;
  flushSync(() => {
    viewStore.set(viewForType(type, payload));
    openStore.set(true);
  });
  const opened = document.getElementById('todo-task-dialog');
  if (!opened) return;
  opened.dataset.dialogType = type;
  dualWriteOpen(true);
  queueMicrotask(() => {
    document
      .querySelector<HTMLElement>('#todo-task-dialog-body input.todo-task-dialog-field, #todo-task-dialog-body [data-sub-row-input]')
      ?.focus();
  });

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

export async function handleSubmit(type: TodoTaskDialogType) {
  if (!submitHandler) return;
  const view = viewStore.getSnapshot();

  if (type === 'create-master') {
    const leftoverTitle = (
      document.querySelector('#todo-task-dialog-body [data-field="title"]') as HTMLInputElement | null
    )?.value;
    const title = (view.titleField || leftoverTitle || '').trim();
    if (!title) {
      patchView({ error: 'Please enter a todo name' });
      return;
    }
    const leftoverSubs = [
      ...document.querySelectorAll('#todo-task-dialog-body [data-sub-row-input]'),
    ].map((node) => (node as HTMLInputElement).value.trim());
    const fromStore = view.subRows.map((row) => row.trim()).filter(Boolean);
    const subTitles = fromStore.length ? fromStore : leftoverSubs.filter(Boolean);
    patchView({ loading: true, error: '' });
    try {
      await submitHandler({ title, subTitles: subTitles.length ? subTitles : undefined });
      closeTodoTaskDialog();
    } catch (err) {
      patchView({ error: (err as Error)?.message || 'Operation failed', loading: false });
    }
    return;
  }

  if (type === 'create-category') {
    const leftoverName = (
      document.querySelector('#todo-task-dialog-body [data-field="name"]') as HTMLInputElement | null
    )?.value;
    const name = (view.nameField || leftoverName || '').trim();
    if (!name) {
      patchView({ error: 'Please enter a category name' });
      return;
    }
    patchView({ loading: true, error: '' });
    try {
      await submitHandler({ name });
      closeTodoTaskDialog();
    } catch (err) {
      patchView({ error: (err as Error)?.message || 'Failed to create category', loading: false });
    }
    return;
  }

  if (type === 'add-sub') {
    const leftoverSubTitle = (
      document.querySelector('#todo-task-dialog-body [data-field="title"]') as HTMLInputElement | null
    )?.value;
    const title = (view.subTitleField || leftoverSubTitle || '').trim();
    if (!title) {
      patchView({ error: 'Please enter a sub-task name' });
      return;
    }
    patchView({ loading: true, error: '' });
    try {
      await submitHandler({ title });
      closeTodoTaskDialog();
    } catch (err) {
      patchView({ error: (err as Error)?.message || 'Operation failed', loading: false });
    }
    return;
  }

  patchView({ loading: true, error: '' });
  try {
    await submitHandler({});
    closeTodoTaskDialog();
  } catch (err) {
    patchView({ error: (err as Error)?.message || 'Operation failed', loading: false });
  }
}
