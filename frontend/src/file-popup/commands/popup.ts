import { asRecord } from '../../host/api-types.ts';
import { readGet, writePost } from '../../host/api/transport.ts';
import {
  emptyFilePopupView,
  layerFromAbsPath,
  patchView,
  titleFromPath,
  viewStore,
} from '../state/store.ts';

export type OpenFilePopupInput = {
  path: string;
  title?: string;
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

export async function openFilePopup({ path, title }: OpenFilePopupInput) {
  const abs = String(path || '').trim();
  if (!abs) return;
  const layer = layerFromAbsPath(abs);
  patchView({
    open: true,
    path: abs,
    title: titleFromPath(abs, title),
    layer,
    content: '',
    editing: false,
    loading: true,
    saving: false,
    error: '',
  });
  try {
    const data = await readGet(
      `/api/file?path=${encodeURIComponent(abs)}&layer=${encodeURIComponent(layer)}&_=${Date.now()}`,
    );
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
      layer: view.layer,
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
