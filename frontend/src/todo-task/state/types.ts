export type TodoSub = {
  sub_task_id: string;
  title?: string;
  status?: string;
  content?: string;
  linked_archive_ids?: string[];
};

export type TodoMaster = {
  master_task_id: string;
  title?: string;
  status?: string;
  created_at?: number | string;
  category_id?: string;
  sub_tasks?: TodoSub[];
  todo_md?: string;
  migration_error?: unknown;
};

export type TodoCategory = {
  id: string;
  name?: string;
  is_default?: boolean;
};

export type TodoComment = {
  id: string;
  body?: string;
  created_at?: string;
};

export type TodoAttachment = {
  file_name: string;
  added_at?: string;
  path?: string;
};

export type StagedAttachment = {
  source_path?: string;
  sourcePath?: string;
};

export type TodoPageCtx = {
  isDisposed: () => boolean;
  isBusy: () => boolean;
  setBusy: (value: boolean) => void;
  paint: () => void;
  getSelectedMasterId: () => string;
  getContainer: () => HTMLElement;
  findSelectedMaster: () => TodoMaster | null;
  reloadList: (options?: { afterWrite?: boolean }) => Promise<void>;
  getCategories: () => TodoCategory[];
};

export function errMessage(err: unknown, fallback: string): string {
  if (err && typeof err === 'object' && 'message' in err) {
    const msg = (err as { message?: unknown }).message;
    if (typeof msg === 'string' && msg) return msg;
  }
  return fallback;
}

/** Node stubs may lack HTMLTextAreaElement; prefer a value-field check. */
export function elementValue(el: Element | null): string {
  return el && 'value' in el ? String((el as { value: unknown }).value ?? '') : '';
}
