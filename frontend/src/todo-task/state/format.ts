import { escHtml } from '../../shared/utils.ts';
import { DEFAULT_PLAN_CATEGORY_ID } from './host.ts';
import type { TodoCategory, TodoMaster, TodoSub } from './types.ts';

export const COPY_MASTER_LABEL = 'Copy';
export const COPY_MASTER_TITLE = 'Copy title and ID';
export const COPY_FEEDBACK_LABEL = '✓ Copied';
export const COPY_FEEDBACK_MS = 1200;
export const COPY_ID_PREFIX = 'todo_';

const STATUS_LABELS: Record<string, string> = {
  incomplete: 'In progress',
  complete: 'Completed',
  abandoned: 'Abandoned',
};

type CopyBtn = HTMLElement & { _copyFeedbackTimer?: ReturnType<typeof setTimeout> | null };

export function formatCopyId(rawId: unknown) {
  const id = String(rawId ?? '').trim();
  if (!id || id.startsWith(COPY_ID_PREFIX)) return id;
  return `${COPY_ID_PREFIX}${id}`;
}

export function copySubIdPair(masterId: unknown, subId: unknown) {
  return `${formatCopyId(masterId)} → ${formatCopyId(subId)}`;
}

/** Clipboard payload for a todo: title + ID, English labels, one field per line. */
export function formatMasterCopyText(title: unknown, masterTaskId: unknown) {
  const t = String(title ?? '').trim() || '(untitled)';
  return `Title: ${t}\nID: ${formatCopyId(masterTaskId)}`;
}

export function escCopyDataAttr(text: unknown) {
  return escHtml(String(text ?? ''))
    .replace(/"/g, '&quot;')
    .replace(/\n/g, '&#10;');
}

export function formatTodoTaskStatus(status: string) {
  return STATUS_LABELS[status] ?? status;
}

export function buildDeepLink(masterId: string, subId: string) {
  const params = new URLSearchParams({ master: masterId, sub: subId });
  return `#/todo-tasks?${params.toString()}`;
}

export function buildMasterDeepLink(masterId: string) {
  const params = new URLSearchParams({ master: masterId });
  return `#/todo-tasks?${params.toString()}`;
}

export function sortMasters(masters: TodoMaster[]) {
  return [...masters].sort((a, b) => {
    const aTime = Date.parse(String(a.created_at ?? '')) || 0;
    const bTime = Date.parse(String(b.created_at ?? '')) || 0;
    return bTime - aTime;
  });
}

export function pickDefaultSub(master: TodoMaster) {
  const subs = master.sub_tasks ?? [];
  const incomplete = subs.find((sub: TodoSub) => sub.status !== 'complete');
  return incomplete ?? subs[0] ?? null;
}

export function formatRelativeTime(iso: unknown) {
  const time = Date.parse(String(iso ?? ''));
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

export function controlsDisabled(busy: boolean) {
  return busy;
}

export function masterStatusClass(status?: string) {
  const wire = status === 'complete' || status === 'abandoned' ? status : 'incomplete';
  return wire;
}

export function isIncompleteMaster(master?: TodoMaster | null) {
  return masterStatusClass(master?.status) === 'incomplete';
}

export function masterCategoryId(master?: TodoMaster | null) {
  const id = typeof master?.category_id === 'string' ? master.category_id.trim() : '';
  return id || DEFAULT_PLAN_CATEGORY_ID;
}

export function filterMastersForView(
  masters: TodoMaster[],
  activeOnly: boolean,
  categoryId = '',
) {
  let list = masters;
  if (activeOnly) list = list.filter(isIncompleteMaster);
  if (categoryId) {
    list = list.filter((master) => masterCategoryId(master) === categoryId);
  }
  return list;
}

export function isDefaultCategory(category: TodoCategory | undefined, categoryId: string) {
  if (!categoryId) return false;
  if (categoryId === DEFAULT_PLAN_CATEGORY_ID) return true;
  return Boolean(category?.is_default);
}

/** English chrome for the built-in default bucket (Host may store a localized name). */
export function categoryDisplayName(category?: TodoCategory | null) {
  if (!category) return '';
  if (category.id === DEFAULT_PLAN_CATEGORY_ID || category.is_default) {
    return 'Uncategorized';
  }
  return typeof category.name === 'string' ? category.name : '';
}

export function flashCopyFeedback(btn: EventTarget | null, restoreLabel: string) {
  if (!(btn instanceof HTMLElement)) return;
  const copyBtn = btn as CopyBtn;
  const prev = copyBtn._copyFeedbackTimer;
  if (prev) clearTimeout(prev);
  copyBtn.textContent = COPY_FEEDBACK_LABEL;
  copyBtn.classList.remove('todo-task-copy-flash');
  void copyBtn.offsetWidth;
  copyBtn.classList.add('todo-task-copy-flash');
  copyBtn._copyFeedbackTimer = setTimeout(() => {
    copyBtn.textContent = restoreLabel;
    copyBtn.classList.remove('todo-task-copy-flash');
    copyBtn._copyFeedbackTimer = null;
  }, COPY_FEEDBACK_MS);
}
