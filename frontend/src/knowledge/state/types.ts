export type TreeNode = {
  name: string;
  relative_path: string;
  is_dir: boolean;
  expanded: boolean;
  loaded: boolean;
  children: TreeNode[];
};

export type RepoPickerOption = { value: string; label: string; title: string };

export type RepoRecord = { full_name: string };

/** mountKbReader stashes its disposer on the host element. */
export type KbReaderHost = HTMLElement & { _kbUnmount?: () => void };

export type ReaderListener = [EventTarget, string, EventListener];

export function errMessage(err: unknown, fallback: string): string {
  if (err && typeof err === 'object' && 'message' in err) {
    const msg = (err as { message?: unknown }).message;
    if (typeof msg === 'string' && msg) return msg;
  }
  return fallback;
}
