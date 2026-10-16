import { state, getEntryId, notifyState } from '../../state/host.ts';
import { docFileLang } from '../../state/doc-lang.ts';
import { resetEditAreaScroll } from '../../../shared/utils.ts';
import * as api from '../../../host/api.ts';
import { setDocEditMode } from '../../../doc-editor/view.tsx';
import { initHighlightUI } from '../../ui/viewer/body.tsx';
import { setNotePanelTitle, showNoteOutlet } from './outlet.ts';
import type { HostViewerAnnotation } from '../../../host/snapshot-types.ts';
import type { NoteEntry } from '../../state/types.ts';

function asTextArea(el: HTMLElement | null) {
  if (!el || !('value' in el)) return null;
  return el as HTMLTextAreaElement;
}

function formatFileSize(text: string) {
  const bytes = new Blob([text]).size;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function updateLangBar(_entry?: NoteEntry | null) {
  notifyState();
}

export function updateHeaderUrls(_entry?: NoteEntry | null, _layer?: string, _activePath?: string) {
  notifyState();
}

let openDocGen = 0;

export async function openDoc(entry: NoteEntry, layer = 'raw') {
  const gen = ++openDocGen;
  initHighlightUI();
  state.viewer.entry = entry;
  state.viewer.layer = layer;
  state.viewer.annotation = {};
  state.viewer.lang = entry.translations?.zh ? 'zh' : null;
  state.viewer.editing = false;
  state.viewer.saving = false;
  state.viewer.loading = true;
  state.viewer.loadError = '';
  state.viewer.fileSize = '';
  state.viewer.rawText = '';
  state.viewer.outletMode = 'open';
  state.viewer.outletMessage = '';
  state.viewer.bodyPaintKey += 1;
  exitEditMode(false);
  setNotePanelTitle(entry);
  showNoteOutlet('open');
  notifyState();

  const entryId = getEntryId(entry);
  const lang = docFileLang(entry, state.viewer.lang || '', layer);
  const [mdResult, annResult] = await Promise.allSettled([
    api.fetchFileContent(layer, entryId, lang),
    api.fetchAnnotation(entryId).catch(() => ({})),
  ]);
  if (gen !== openDocGen) return;

  if (annResult.status === 'fulfilled') {
    state.viewer.annotation = (annResult.value || {}) as HostViewerAnnotation;
  }

  if (mdResult.status === 'rejected') {
    state.viewer.loading = false;
    const reason = mdResult.reason;
    state.viewer.loadError = reason instanceof Error ? reason.message : 'fetch failed';
    state.viewer.bodyPaintKey += 1;
    notifyState();
    return;
  }

  const text = mdResult.value;
  state.viewer.rawText = text;
  state.viewer.fileSize = formatFileSize(text);
  state.viewer.loading = false;
  state.viewer.loadError = '';
  state.viewer.bodyPaintKey += 1;
  notifyState();
}

export async function switchLang(lang: string | null) {
  if (!state.viewer.entry) return;
  const entry = state.viewer.entry;
  const layer = state.viewer.layer;
  const gen = ++openDocGen;

  const body = document.getElementById('md-body');
  const cacheKey = `${getEntryId(entry)}:${layer}`;
  if (body) state.viewer.scrollCache[cacheKey] = body.scrollTop;

  state.viewer.lang = lang;
  state.viewer.loading = true;
  state.viewer.loadError = '';
  state.viewer.bodyPaintKey += 1;
  notifyState();

  const langKey = docFileLang(entry, lang || '', layer);
  try {
    const text = await api.fetchFileContent(layer, getEntryId(entry), langKey);
    if (gen !== openDocGen) return;
    state.viewer.rawText = text;
    state.viewer.fileSize = formatFileSize(text);
    state.viewer.loading = false;
    state.viewer.bodyPaintKey += 1;
    notifyState();
    const saved = state.viewer.scrollCache[cacheKey];
    if (saved != null) {
      requestAnimationFrame(() => {
        const next = document.getElementById('md-body');
        if (next) next.scrollTop = saved;
      });
    }
  } catch (e) {
    if (gen !== openDocGen) return;
    state.viewer.loading = false;
    state.viewer.loadError = e instanceof Error ? e.message : String(e);
    state.viewer.bodyPaintKey += 1;
    notifyState();
  }
}

function setEditChromeDisplay(editing: boolean) {
  const btnSave = document.getElementById('btn-save');
  const btnCancel = document.getElementById('btn-cancel-edit');
  const btnEdit = document.getElementById('btn-edit');
  if (btnSave) btnSave.style.display = editing ? '' : 'none';
  if (btnCancel) btnCancel.style.display = editing ? '' : 'none';
  if (btnEdit) btnEdit.style.display = editing ? 'none' : '';
}

export function enterEditMode() {
  state.viewer.editing = true;
  notifyState();
  const editArea = asTextArea(document.getElementById('md-edit-area'));
  const body = document.getElementById('md-body');
  if (body && editArea) {
    setDocEditMode({ bodyEl: body, editAreaEl: editArea, text: state.viewer.rawText, editing: true });
    resetEditAreaScroll(editArea, { focus: true });
  }
  setEditChromeDisplay(true);
}

export function exitEditMode(rerender = true) {
  state.viewer.editing = false;
  const editArea = asTextArea(document.getElementById('md-edit-area'));
  const body = document.getElementById('md-body');
  if (body && editArea) {
    setDocEditMode({ bodyEl: body, editAreaEl: editArea, editing: false });
  }
  setEditChromeDisplay(false);
  if (rerender && state.viewer.entry) {
    state.viewer.bodyPaintKey += 1;
  }
  notifyState();
}

export async function saveDoc() {
  if (!state.viewer.entry) return;
  const editArea = asTextArea(document.getElementById('md-edit-area'));
  const newContent = editArea?.value ?? state.viewer.rawText;
  state.viewer.saving = true;
  notifyState();

  try {
    const lang = docFileLang(state.viewer.entry, state.viewer.lang || '', state.viewer.layer);
    const data = (await api.saveFile(state.viewer.layer, getEntryId(state.viewer.entry), newContent, lang)) as { error?: string };
    if (data.error) throw new Error(data.error);

    state.viewer.rawText = newContent;
    const date = (state.viewer.entry.created_at || '').slice(0, 8);
    const h1Match = newContent.match(/^#\s+(.+)/m);
    const newTitle = h1Match ? h1Match[1].trim() : (state.viewer.entry.title || 'Untitled note');
    const entryId = getEntryId(state.viewer.entry);
    const cache = state.index.titleCache.get(date) ?? new Map<string, string>();
    state.index.titleCache.set(date, cache);
    if (entryId) cache.set(entryId, newTitle);
    exitEditMode(true);
    notifyState();
  } catch (e) {
    alert(`Save failed: ${e instanceof Error ? e.message : String(e)}`);
  } finally {
    state.viewer.saving = false;
    notifyState();
  }
}
