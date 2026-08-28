// @ts-nocheck
/**
 * Todos page lifecycle: enter / selection / leave stay exported.
 * Page path no longer Sets or Resets Host Binding (process-level workbench Set).
 */

/** Explicit leave chain (page path). Binding Reset is no longer part of leave. */
export const TODOS_EXPLICIT_LEAVE_RESET_CHAIN = Object.freeze([
  'frontend/src/todo-task/index.js::dispose',
  'frontend/src/todo-task/lifecycle.js::onTodosPageLeave',
]);

function normalizeMasterId(id) {
  if (typeof id !== 'string') return '';
  return id.trim();
}

/**
 * @param {{ onBound?: Function, onUnbound?: Function, onError?: Function }} [callbacks]
 */
export function createTodosPageLifecycle(_callbacks = {}) {
  let selectedMasterId = null;

  /**
   * Enter Todos page. Binding is process-level; this is a page-lifecycle hook only.
   * @param {string} [selectedId]
   */
  async function onTodosPageEnter(selectedId = '') {
    const id = normalizeMasterId(selectedId);
    selectedMasterId = id || null;
    return { ok: true, skipped: 'page_binding_removed', selectedMasterId };
  }

  /**
   * In-page master change. Does not Present or write Host Binding.
   * @param {string} nextId
   */
  async function onMasterSelectionChange(nextId) {
    const id = normalizeMasterId(nextId);
    if (!id) {
      return { ok: false, skipped: 'empty_context' };
    }
    if (selectedMasterId === id) {
      return { ok: true, selectedMasterId, skipped: 'same_master' };
    }
    selectedMasterId = id;
    return { ok: true, skipped: 'page_binding_removed', selectedMasterId };
  }

  /**
   * Leave Todos page / dispose. Must not be called from shell-close.
   * Does not Reset Host Binding.
   */
  async function onTodosPageLeave() {
    selectedMasterId = null;
    return { ok: true, skipped: 'page_binding_removed' };
  }

  /**
   * Shell close notification. 关壳 ≠ Reset — Binding stays.
   */
  function notifyShellClose() {
    return {
      ok: true,
      reset: false,
      selectedMasterId,
    };
  }

  return {
    onTodosPageEnter,
    onMasterSelectionChange,
    onTodosPageLeave,
    notifyShellClose,
    isBound: () => false,
    getBoundMasterId: () => selectedMasterId,
  };
}

/** Module-default lifecycle for exported function-spec names. */
const defaultLifecycle = createTodosPageLifecycle();

/** @param {string} [selectedMasterId] */
export function onTodosPageEnter(selectedMasterId) {
  return defaultLifecycle.onTodosPageEnter(selectedMasterId);
}

/** @param {string} masterTaskId */
export function onMasterSelectionChange(masterTaskId) {
  return defaultLifecycle.onMasterSelectionChange(masterTaskId);
}

export function onTodosPageLeave() {
  return defaultLifecycle.onTodosPageLeave();
}
