import { state } from '../../state/host.ts';
import { resetEditAreaScroll } from '../../../shared/utils.ts';
import * as api from '../../../host/api.ts';
import { initKbComments } from '../../ui/comments.tsx';
import { setDocEditMode } from '../../../doc-editor/view.tsx';
import { kbHidePendingBadge, kbShowPendingBadge, onKbDirty } from '../../ui/viewer/chrome.tsx';
import { initKbHighlightUI } from '../../ui/viewer/highlight.ts';
import { revokeKbBlobUrls } from './images.ts';
import { paintKbMdBody } from './paint.ts';
import { paintKbDocError, paintKbDocLoading, paintKbDocPlain } from '../../ui/viewer/doc.tsx';

type KbViewer = {
  entry: unknown;
  isKb: boolean;
  kbRepo: string | null;
  kbPath: string | null;
  layer: string;
  rawText: string;
  annotation: Record<string, unknown>;
  lang: unknown;
};

function viewer(): KbViewer {
  return state.viewer as KbViewer;
}

export async function openKbDoc(kbHit: { repo: string; path: string; url?: string }) {
  const { repo, path, url } = kbHit;
  const v = viewer();

  v.entry = null;
  v.isKb = true;
  v.kbRepo = repo;
  v.kbPath = path;
  v.layer = 'raw';
  v.rawText = '';
  v.annotation = {};
  v.lang = null;

  const repoName = (repo || '').split('/').pop();
  const copyHttp = document.getElementById('kb-btn-copy-http') as HTMLElement | null;
  if (copyHttp) {
    copyHttp.dataset.url = url || '';
    copyHttp.dataset.tip = url || '';
  }
  const localPath = state.ui.knowledgeRoot
    ? `${state.ui.knowledgeRoot}/${repoName}/${path}`
    : `${repoName}/${path}`;
  const copyPath = document.getElementById('kb-btn-copy-path') as HTMLElement | null;
  if (copyPath) {
    copyPath.dataset.path = localPath;
    copyPath.dataset.tip = localPath;
  }
  const fileSize = document.getElementById('kb-md-file-size');
  if (fileSize) fileSize.textContent = '';

  const tagsBar = document.getElementById('md-tags-bar');
  if (tagsBar) tagsBar.style.display = 'none';

  const modal = document.getElementById('kb-md-modal');
  const body = document.getElementById('kb-md-body');
  if (!body) return;
  paintKbDocLoading(body);
  if (modal) modal.style.display = 'flex';
  document.body.style.overflow = 'hidden';

  try {
    const [mdResult, annResult] = await Promise.allSettled([
      api.fetchKbFileContent(repo, path),
      api.fetchKbAnnotation(repo, path),
    ]);

    const ann = annResult.status === 'fulfilled' ? annResult.value || {} : {};
    v.annotation = ann as Record<string, unknown>;

    if (mdResult.status === 'rejected') {
      throw new Error((mdResult.reason as Error | undefined)?.message || 'fetch failed');
    }
    const result = mdResult.value as { error?: string; content?: string };
    if (result.error) throw new Error(result.error);

    const text = result.content || '';
    v.rawText = text;

    const bytes = new Blob([text]).size;
    if (fileSize) {
      fileSize.textContent =
        bytes < 1024
          ? `${bytes} B`
          : bytes < 1024 * 1024
            ? `${(bytes / 1024).toFixed(1)} KB`
            : `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    }

    if (typeof marked !== 'undefined') {
      await paintKbMdBody(text);
    } else {
      paintKbDocPlain(body, text);
    }

    const editBtn = document.getElementById('kb-btn-edit');
    if (editBtn) editBtn.style.display = '';

    initKbComments();
    initKbHighlightUI();

    api
      .fetchKbStatus(repo)
      .then((data: { error?: string; total?: number; ahead?: number }) => {
        if (!data.error && ((data.total ?? 0) > 0 || (data.ahead ?? 0) > 0)) {
          kbShowPendingBadge('chore: update via viewer');
        }
      })
      .catch(() => {});

    document.addEventListener('kb:dirty', onKbDirty);
  } catch (err) {
    paintKbDocError(body, err instanceof Error ? err.message : String(err));
    const editBtn = document.getElementById('kb-btn-edit');
    if (editBtn) editBtn.style.display = 'none';
  }
}

export async function saveKbDoc() {
  const editArea = document.getElementById('kb-md-edit-area') as HTMLTextAreaElement | null;
  const btnSave = document.getElementById('kb-btn-save') as HTMLButtonElement | null;
  if (!editArea || !btnSave) return;
  const newContent = editArea.value;
  btnSave.disabled = true;
  btnSave.textContent = 'Saving…';
  try {
    const v = viewer();
    const originalContent = v.rawText;
    const data = (await api.saveKbFile(v.kbRepo, v.kbPath, newContent)) as { error?: string };
    if (data.error) throw new Error(data.error);
    v.rawText = newContent;
    _kbExitEditMode();

    const body = document.getElementById('kb-md-body');
    if (body) {
      if (typeof marked !== 'undefined') {
        await paintKbMdBody(newContent);
      } else {
        paintKbDocPlain(body, newContent);
      }
    }
    const editBtn = document.getElementById('kb-btn-edit');
    if (editBtn) editBtn.style.display = '';
    if (newContent !== originalContent) {
      kbShowPendingBadge('update: edit via viewer');
    }
  } catch (err) {
    alert(`Save failed: ${err instanceof Error ? err.message : String(err)}`);
  } finally {
    btnSave.disabled = false;
    btnSave.textContent = '💾 Save';
  }
}

export function closeKbModal() {
  const modal = document.getElementById('kb-md-modal');
  if (modal) modal.style.display = 'none';
  document.removeEventListener('kb:dirty', onKbDirty);
  revokeKbBlobUrls();
  const v = viewer();
  v.isKb = false;
  v.kbRepo = null;
  v.kbPath = null;
  v.annotation = {};
  document.body.style.overflow = '';
  kbHidePendingBadge();
}

export function _kbEnterEditMode() {
  const body = document.getElementById('kb-md-body');
  const editArea = document.getElementById('kb-md-edit-area') as HTMLTextAreaElement | null;
  setDocEditMode({
    bodyEl: body,
    editAreaEl: editArea,
    text: viewer().rawText,
    editing: true,
  });
  const btnSave = document.getElementById('kb-btn-save');
  const btnCancel = document.getElementById('kb-btn-cancel-edit');
  const btnEdit = document.getElementById('kb-btn-edit');
  if (btnSave) btnSave.style.display = '';
  if (btnCancel) btnCancel.style.display = '';
  if (btnEdit) btnEdit.style.display = 'none';
  resetEditAreaScroll(editArea, { focus: true });
}

export function _kbExitEditMode() {
  const body = document.getElementById('kb-md-body');
  const editArea = document.getElementById('kb-md-edit-area');
  if (!(body instanceof HTMLElement) || !(editArea instanceof HTMLTextAreaElement)) return;
  setDocEditMode({ bodyEl: body, editAreaEl: editArea, text: viewer().rawText, editing: false });
  const btnSave = document.getElementById('kb-btn-save');
  const btnCancel = document.getElementById('kb-btn-cancel-edit');
  const btnEdit = document.getElementById('kb-btn-edit');
  if (btnSave) btnSave.style.display = 'none';
  if (btnCancel) btnCancel.style.display = 'none';
  if (btnEdit) btnEdit.style.display = '';
}
