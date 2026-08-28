// @ts-nocheck
import { escHtml } from '../shared/utils.ts';
import { DEFAULT_PLAN_CATEGORY_ID } from './host.ts';

export const COPY_MASTER_LABEL = 'Copy';
export const COPY_MASTER_TITLE = 'Copy title and ID';
export const COPY_FEEDBACK_LABEL = '✓ Copied';
export const COPY_FEEDBACK_MS = 1200;
export const COPY_ID_PREFIX = 'todo_';

const STATUS_LABELS = {
  incomplete: 'In progress',
  complete: 'Completed',
  abandoned: 'Abandoned',
};

export function formatCopyId(rawId) {
  const id = String(rawId ?? '').trim();
  if (!id || id.startsWith(COPY_ID_PREFIX)) return id;
  return `${COPY_ID_PREFIX}${id}`;
}

export function copySubIdPair(masterId, subId) {
  return `${formatCopyId(masterId)} → ${formatCopyId(subId)}`;
}

/** Clipboard payload for a todo: title + ID, English labels, one field per line. */
export function formatMasterCopyText(title, masterTaskId) {
  const t = String(title ?? '').trim() || '(untitled)';
  return `Title: ${t}\nID: ${formatCopyId(masterTaskId)}`;
}

export function escCopyDataAttr(text) {
  return escHtml(String(text ?? ''))
    .replace(/"/g, '&quot;')
    .replace(/\n/g, '&#10;');
}

export function formatTodoTaskStatus(status) {
  return STATUS_LABELS[status] ?? status;
}

export function buildDeepLink(masterId, subId) {
  const params = new URLSearchParams({ master: masterId, sub: subId });
  return `#/todo-tasks?${params.toString()}`;
}

export function buildMasterDeepLink(masterId) {
  const params = new URLSearchParams({ master: masterId });
  return `#/todo-tasks?${params.toString()}`;
}

export function sortMasters(masters) {
  return [...masters].sort((a, b) => {
    const aTime = Date.parse(a.created_at ?? '') || 0;
    const bTime = Date.parse(b.created_at ?? '') || 0;
    return bTime - aTime;
  });
}

export function pickDefaultSub(master) {
  const subs = master.sub_tasks ?? [];
  const incomplete = subs.find((sub) => sub.status !== 'complete');
  return incomplete ?? subs[0] ?? null;
}

export function formatRelativeTime(iso) {
  const time = Date.parse(iso ?? '');
  if (!time) return '';
  const diffMs = Date.now() - time;
  const minutes = Math.floor(diffMs / 60000);
  if (minutes < 1) return 'Just now';
  if (minutes < 60) return `${minutes} minutes ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} hours ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days} days ago`;
  return new Date(time).toLocaleDateString('en-US');
}

export function controlsDisabled(busy) {
  return busy;
}

export function masterStatusClass(status) {
  const wire = status === 'complete' || status === 'abandoned' ? status : 'incomplete';
  return wire;
}

export function isIncompleteMaster(master) {
  return masterStatusClass(master?.status) === 'incomplete';
}

export function masterCategoryId(master) {
  const id = typeof master?.category_id === 'string' ? master.category_id.trim() : '';
  return id || DEFAULT_PLAN_CATEGORY_ID;
}

export function filterMastersForView(masters, activeOnly, categoryId = '') {
  let list = masters;
  if (activeOnly) list = list.filter(isIncompleteMaster);
  if (categoryId) {
    list = list.filter((master) => masterCategoryId(master) === categoryId);
  }
  return list;
}

export function isDefaultCategory(category, categoryId) {
  if (!categoryId) return false;
  if (categoryId === DEFAULT_PLAN_CATEGORY_ID) return true;
  return Boolean(category?.is_default);
}

/** English chrome for the built-in default bucket (Host may store a localized name). */
export function categoryDisplayName(category) {
  if (!category) return '';
  if (category.id === DEFAULT_PLAN_CATEGORY_ID || category.is_default) {
    return 'Uncategorized';
  }
  return typeof category.name === 'string' ? category.name : '';
}

export function flashCopyFeedback(btn, restoreLabel) {
  if (!(btn instanceof HTMLElement)) return;
  const prev = btn._copyFeedbackTimer;
  if (prev) clearTimeout(prev);
  btn.textContent = COPY_FEEDBACK_LABEL;
  btn.classList.remove('todo-task-copy-flash');
  void btn.offsetWidth;
  btn.classList.add('todo-task-copy-flash');
  btn._copyFeedbackTimer = setTimeout(() => {
    btn.textContent = restoreLabel;
    btn.classList.remove('todo-task-copy-flash');
    btn._copyFeedbackTimer = null;
  }, COPY_FEEDBACK_MS);
}
