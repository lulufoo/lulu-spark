import { createModuleStore } from '../../shared/module-store.ts';
import { filenameFromPath } from '../../shared/utils.ts';

export type FilePopupView = {
  open: boolean;
  path: string;
  title: string;
  layer: 'raw' | 'digest';
  content: string;
  editing: boolean;
  loading: boolean;
  saving: boolean;
  error: string;
};

export function emptyFilePopupView(): FilePopupView {
  return {
    open: false,
    path: '',
    title: '',
    layer: 'raw',
    content: '',
    editing: false,
    loading: false,
    saving: false,
    error: '',
  };
}

export const viewStore = createModuleStore<FilePopupView>(emptyFilePopupView());

export function patchView(partial: Partial<FilePopupView>) {
  viewStore.set({ ...viewStore.getSnapshot(), ...partial });
}

export function titleFromPath(path: string, title?: string) {
  const given = title?.trim();
  if (given) return given;
  return filenameFromPath(path.replace(/\\/g, '/')) || path;
}

export function layerFromAbsPath(path: string): 'raw' | 'digest' {
  const parts = path.split(/[/\\]/).filter(Boolean);
  for (let i = parts.length - 1; i >= 0; i--) {
    if (parts[i] === 'digest') return 'digest';
    if (parts[i] === 'raw') return 'raw';
  }
  return 'raw';
}

/** Notes tree after `/notes/raw/` or `/notes/digest/`. Empty when the path is not a note. */
export function commonPathFromAbsPath(path: string): string {
  const normalized = path.replace(/\\/g, '/');
  for (const layer of ['raw', 'digest'] as const) {
    const marker = `/notes/${layer}/`;
    const idx = normalized.lastIndexOf(marker);
    if (idx >= 0) return normalized.slice(idx + marker.length);
  }
  return '';
}
