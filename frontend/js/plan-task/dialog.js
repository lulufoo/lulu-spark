import { escHtml } from '../utils.js';

/** @typedef {'create-master' | 'create-category' | 'add-sub' | 'delete-master' | 'delete-sub'} PlanTaskDialogType */

/** @type {HTMLElement | null} */
let lastTrigger = null;
/** @type {((values: Record<string, unknown>) => Promise<void>) | null} */
let submitHandler = null;
/** @type {((event: KeyboardEvent) => void) | null} */
let keydownHandler = null;

function getDialogEl() {
  return document.getElementById('plan-task-dialog');
}

function getBodyEl() {
  return document.getElementById('plan-task-dialog-body');
}

function getErrorEl() {
  return document.getElementById('plan-task-dialog-error');
}

function getPrimaryBtn() {
  return document.getElementById('plan-task-dialog-primary');
}

function getCancelBtn() {
  return document.getElementById('plan-task-dialog-cancel');
}

function getTitleEl() {
  return document.getElementById('plan-task-dialog-title');
}

function setDialogError(message) {
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

function setSubmitLoading(loading) {
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
    .map((input) => input.value.trim())
    .filter(Boolean);
}

function renderSubTitleRows(rows = ['']) {
  return `
    <div class="plan-task-dialog-sub-rows" data-sub-rows>
      ${rows
        .map(
          (value, index) => `
        <div class="plan-task-dialog-sub-row">
          <input
            type="text"
            class="plan-task-dialog-field"
            data-sub-row-input
            placeholder="Sub-task ${index + 1}"
            value="${escHtml(value)}"
          />
          <button type="button" class="plan-task-dialog-row-remove" data-action="remove-sub-row" aria-label="Remove"${rows.length <= 1 ? ' disabled' : ''}>×</button>
        </div>`,
        )
        .join('')}
    </div>
    <button type="button" class="plan-task-dialog-add-row" data-action="add-sub-row">+ Add sub-task row</button>
    <p class="plan-task-dialog-hint">If you skip sub-tasks, the todo will have none</p>
  `;
}

/**
 * @param {PlanTaskDialogType} type
 * @param {Record<string, unknown>} payload
 */
function renderDialogBody(type, payload) {
  const body = getBodyEl();
  const title = getTitleEl();
  const primary = getPrimaryBtn();
  if (!body || !title || !primary) return;

  setDialogError('');
  primary.classList.remove('danger');

  if (type === 'create-master') {
    title.textContent = 'New todo';
    primary.textContent = 'Create todo';
    body.innerHTML = `
      <label class="plan-task-dialog-label">
        <span class="plan-task-dialog-label-text">Todo name <span class="plan-task-dialog-required">*</span></span>
        <input type="text" class="plan-task-dialog-field" data-field="title" placeholder="e.g. FM-4 UI polish" />
      </label>
      <div class="plan-task-dialog-section">
        <span class="plan-task-dialog-label-text">Initial sub-tasks (optional)</span>
        ${renderSubTitleRows([''])}
      </div>
    `;
    return;
  }

  if (type === 'create-category') {
    title.textContent = 'New category';
    primary.textContent = 'Create category';
    body.innerHTML = `
      <p class="plan-task-dialog-lead">
        Organize todos under a shared category. The name is available in the Todos filter and MCP tools.
      </p>
      <label class="plan-task-dialog-label">
        <span class="plan-task-dialog-label-text">Category name <span class="plan-task-dialog-required">*</span></span>
        <input
          type="text"
          class="plan-task-dialog-field"
          data-field="name"
          maxlength="64"
          autocomplete="off"
          spellcheck="false"
          placeholder="e.g. Product launch"
        />
      </label>
      <p class="plan-task-dialog-hint">Use a short, unique name. You can delete empty categories later.</p>
    `;
    return;
  }

  if (type === 'add-sub') {
    title.textContent = 'Add sub-task';
    primary.textContent = 'Add';
    body.innerHTML = `
      <p class="plan-task-dialog-readonly">Parent todo:${escHtml(String(payload.masterTitle ?? ''))}</p>
      <label class="plan-task-dialog-label">
        <span class="plan-task-dialog-label-text">Sub-task name <span class="plan-task-dialog-required">*</span></span>
        <input type="text" class="plan-task-dialog-field" data-field="title" placeholder="Required" />
      </label>
    `;
    return;
  }

  if (type === 'delete-master') {
    title.textContent = 'Delete todo?';
    primary.textContent = 'Delete todo';
    primary.classList.add('danger');
    const subCount = Number(payload.subCount ?? 0);
    body.innerHTML = `
      <p class="plan-task-dialog-message">
        Permanently delete “${escHtml(String(payload.masterTitle ?? ''))}” and its ${subCount} sub-tasks.
        This cannot be undone.
      </p>
    `;
    return;
  }

  if (type === 'delete-sub') {
    title.textContent = 'Delete sub-task?';
    primary.textContent = 'Delete';
    primary.classList.add('danger');
    body.innerHTML = `
      <p class="plan-task-dialog-message">
        Delete “${escHtml(String(payload.subTitle ?? ''))}”.
      </p>
    `;
  }
}

function focusFirstField() {
  const body = getBodyEl();
  const first = body?.querySelector('input.plan-task-dialog-field, [data-sub-row-input]');
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

export function closePlanTaskDialog() {
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
  document.dispatchEvent(new CustomEvent('plan-task-dialog-close'));
}

export function isPlanTaskDialogOpen() {
  return getDialogEl()?.classList.contains('open') ?? false;
}

/**
 * @param {{
 *   type: PlanTaskDialogType,
 *   payload?: Record<string, unknown>,
 *   onSubmit: (values: Record<string, unknown>) => Promise<void>,
 *   triggerEl?: HTMLElement | null,
 * }} options
 */
export function openPlanTaskDialog({ type, payload = {}, onSubmit, triggerEl = null }) {
  wirePlanTaskDialog();
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
      closePlanTaskDialog();
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

async function handleSubmit(type) {
  if (!submitHandler) return;
  const body = getBodyEl();
  if (!body) return;

  if (type === 'create-master') {
    const title = body.querySelector('[data-field="title"]')?.value?.trim() ?? '';
    if (!title) {
      setDialogError('Please enter a todo name');
      return;
    }
    const subTitles = collectSubTitleRows();
    setSubmitLoading(true);
    setDialogError('');
    try {
      await submitHandler({ title, subTitles: subTitles.length ? subTitles : undefined });
      closePlanTaskDialog();
    } catch (err) {
      setDialogError(err?.message || 'Operation failed');
      setSubmitLoading(false);
    }
    return;
  }

  if (type === 'create-category') {
    const name = body.querySelector('[data-field="name"]')?.value?.trim() ?? '';
    if (!name) {
      setDialogError('Please enter a category name');
      return;
    }
    setSubmitLoading(true);
    setDialogError('');
    try {
      await submitHandler({ name });
      closePlanTaskDialog();
    } catch (err) {
      setDialogError(err?.message || 'Failed to create category');
      setSubmitLoading(false);
    }
    return;
  }

  if (type === 'add-sub') {
    const title = body.querySelector('[data-field="title"]')?.value?.trim() ?? '';
    if (!title) {
      setDialogError('Please enter a sub-task name');
      return;
    }
    setSubmitLoading(true);
    setDialogError('');
    try {
      await submitHandler({ title });
      closePlanTaskDialog();
    } catch (err) {
      setDialogError(err?.message || 'Operation failed');
      setSubmitLoading(false);
    }
    return;
  }

  setSubmitLoading(true);
  setDialogError('');
  try {
    await submitHandler({});
    closePlanTaskDialog();
  } catch (err) {
    setDialogError(err?.message || 'Operation failed');
    setSubmitLoading(false);
  }
}

function wirePlanTaskDialog() {
  const dialog = getDialogEl();
  if (!dialog || dialog.dataset.wired === '1') return;
  dialog.dataset.wired = '1';

  dialog.addEventListener('click', (event) => {
    if (event.target === dialog) {
      closePlanTaskDialog();
    }
  });

  getCancelBtn()?.addEventListener('click', () => closePlanTaskDialog());
  getPrimaryBtn()?.addEventListener('click', () => {
    const type = /** @type {PlanTaskDialogType | undefined} */ (dialog.dataset.dialogType);
    if (type) void handleSubmit(type);
  });

  dialog.addEventListener('click', (event) => {
    const actionEl = event.target.closest('[data-action]');
    const action = actionEl?.dataset.action;
    if (action === 'add-sub-row') {
      event.preventDefault();
      const container = dialog.querySelector('[data-sub-rows]');
      if (!container) return;
      const rows = collectSubTitleRows();
      rows.push('');
      container.outerHTML = renderSubTitleRows(rows);
      const inputs = dialog.querySelectorAll('[data-sub-row-input]');
      const last = inputs[inputs.length - 1];
      if (last instanceof HTMLElement) last.focus();
      return;
    }
    if (action === 'remove-sub-row') {
      event.preventDefault();
      const row = actionEl?.closest('.plan-task-dialog-sub-row');
      const input = row?.querySelector('[data-sub-row-input]');
      const container = dialog.querySelector('[data-sub-rows]');
      if (!container || !(input instanceof HTMLInputElement)) return;
      const allInputs = [...dialog.querySelectorAll('[data-sub-row-input]')];
      const rows = allInputs
        .filter((el) => el !== input)
        .map((el) => el.value.trim());
      container.outerHTML = renderSubTitleRows(rows.length ? rows : ['']);
    }
  });
}

wirePlanTaskDialog();
