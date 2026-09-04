import { asRecord } from '../../host/api-types.ts';
import { readGet, writePost } from '../../host/api/transport.ts';
import {
  emptyFilePopupView,
  patchView,
  titleFromPath,
  viewStore,
} from '../state/store.ts';

export type OpenFilePopupInput = {
  path: string;
  title?: string;
  identityKey?: string;
};

function asTextArea(el: HTMLElement | null) {
  if (!el || !('value' in el)) return null;
  return el as HTMLTextAreaElement;
}

export function closeFilePopup() {
  viewStore.set(emptyFilePopupView());
}

export function enterFilePopupEdit() {
  const view = viewStore.getSnapshot();
  if (!view.open || view.loading || view.saving) return;
  patchView({ editing: true, error: '' });
}

export function cancelFilePopupEdit() {
  const view = viewStore.getSnapshot();
  if (!view.open || view.saving) return;
  patchView({ editing: false, error: '' });
}

/** Copy the open document's absolute path. Returns false when there is nothing to write. */
export async function copyFilePopupPath() {
  const abs = String(viewStore.getSnapshot().path || '').trim();
  if (!abs) return false;
  if (!navigator.clipboard?.writeText) return false;
  try {
    await navigator.clipboard.writeText(abs);
    return true;
  } catch {
    return false;
  }
}

export async function openFilePopup({ path, title, identityKey }: OpenFilePopupInput) {
  const abs = String(path || '').trim();
  if (!abs) return;
  const key = String(identityKey || '').trim();
  patchView({
    open: true,
    path: abs,
    title: titleFromPath(abs, title),
    identityKey: key,
    content: '',
    editing: false,
    loading: true,
    saving: false,
    error: '',
  });
  try {
    const data = await readGet(`/api/file?path=${encodeURIComponent(abs)}&_=${Date.now()}`);
    const current = viewStore.getSnapshot();
    if (!current.open || current.path !== abs) return;
    const content =
      typeof data === 'string' ? data : String(asRecord(data)?.content ?? '');
    patchView({ content, loading: false, error: '' });
  } catch (err) {
    const current = viewStore.getSnapshot();
    if (!current.open || current.path !== abs) return;
    patchView({
      loading: false,
      error: err instanceof Error ? err.message : 'Failed to load file',
    });
  }
}

export async function saveFilePopup() {
  const view = viewStore.getSnapshot();
  if (!view.open || !view.path || view.loading || view.saving) return;
  const editArea = asTextArea(document.getElementById('file-popup-edit-area'));
  const content = editArea?.value ?? view.content;
  patchView({ saving: true, error: '' });
  try {
    await writePost('/api/file', {
      path: view.path,
      content,
    });
    const current = viewStore.getSnapshot();
    if (!current.open || current.path !== view.path) return;
    patchView({ content, editing: false, saving: false, error: '' });
  } catch (err) {
    const current = viewStore.getSnapshot();
    if (!current.open || current.path !== view.path) return;
    patchView({
      saving: false,
      error: err instanceof Error ? err.message : 'Save failed',
    });
  }
}
