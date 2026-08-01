/**
 * Todos page lifecycle (SK-2): enter/first-select → Set; in-page change → replace Set;
 * leave → Reset. Shell close ≠ Reset.
 */

import { buildTodosBinding, resetTodosBinding } from './todos-binding.js';

/** Explicit leave→Reset chain (primary). Defensive cut backs missed leave; does not replace. */
export const TODOS_EXPLICIT_LEAVE_RESET_CHAIN = Object.freeze([
  'frontend/js/plan-task/index.js::dispose',
  'frontend/js/plan-task/todos-lifecycle.js::onTodosPageLeave',
  'frontend/js/plan-task/todos-binding.js::resetTodosBinding',
  'src-tauri/src/services/agent/loop.rs::reset_binding',
]);

function normalizeMasterId(masterTaskId) {
  if (typeof masterTaskId !== 'string') return '';
  return masterTaskId.trim();
}

/**
 * @param {{ onBound?: Function, onUnbound?: Function, onError?: Function }} [callbacks]
 */
export function createTodosPageLifecycle(callbacks = {}) {
  let bound = false;
  let boundMasterId = null;

  async function applySet(masterTaskId) {
    const wasBound = bound;
    const result = await buildTodosBinding({ masterTaskId }, callbacks);
    if (result.ok) {
      bound = true;
      boundMasterId = masterTaskId;
      return { ...result, boundMasterId };
    }
    // Replace Set failure: keep old Binding (still bound); do not Reset.
    if (wasBound) {
      return {
        ...result,
        retainedBinding: true,
        boundMasterId,
        state: 'bound',
      };
    }
    return {
      ...result,
      boundMasterId: null,
      state: result.state ?? 'unbound',
    };
  }

  /**
   * Enter Todos page. Sets when a selected master is available; otherwise defers.
   * @param {string} [selectedMasterId]
   */
  async function onTodosPageEnter(selectedMasterId = '') {
    const id = normalizeMasterId(selectedMasterId);
    if (!id) {
      return { ok: false, skipped: 'empty_context', state: 'unbound' };
    }
    return applySet(id);
  }

  /**
   * In-page master change: first available selection Sets; later changes replace Set.
   * Does not Present or close the assistant shell.
   * @param {string} masterTaskId
   */
  async function onMasterSelectionChange(masterTaskId) {
    const id = normalizeMasterId(masterTaskId);
    if (!id) {
      return { ok: false, skipped: 'empty_context' };
    }
    if (bound && boundMasterId === id) {
      return { ok: true, state: 'bound', boundMasterId, skipped: 'same_master' };
    }
    return applySet(id);
  }

  /**
   * Leave Todos page / dispose → Reset → onUnbound.
   * Must not be called from shell-close.
   */
  async function onTodosPageLeave() {
    const result = await resetTodosBinding(callbacks);
    bound = false;
    boundMasterId = null;
    return result;
  }

  /**
   * Shell close notification. 关壳 ≠ Reset — Binding stays.
   */
  function notifyShellClose() {
    return {
      ok: true,
      reset: false,
      state: bound ? 'bound' : 'unbound',
      boundMasterId,
    };
  }

  return {
    onTodosPageEnter,
    onMasterSelectionChange,
    onTodosPageLeave,
    notifyShellClose,
    isBound: () => bound,
    getBoundMasterId: () => boundMasterId,
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
