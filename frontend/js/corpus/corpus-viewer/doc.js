import { state } from '../../host/state.js'
import { escHtml, resetEditAreaScroll } from '../../shared/utils.js'
import * as api from '../../host/api.js'
import { initKbComments } from '../corpus-comments.js'
import { setDocEditMode } from '../../doc-editor/view.js'
import { kbHidePendingBadge, kbShowPendingBadge, onKbDirty, showKbReindexBtn } from './chrome.js'
import { initKbHighlightUI, renderKbMdBody } from './highlight.js'

export async function openKbDoc(kbHit) {
  const { repo, path, url } = kbHit;

  state.viewer.entry = null;
  state.viewer.isKb = true;
  state.viewer.kbRepo = repo;
  state.viewer.kbPath = path;
  state.viewer.layer = 'raw';
  state.viewer.rawText = '';
  state.viewer.annotation = {};
  state.viewer.lang = null;

  const pathParts = (path || '').split('/');
  const fileName = pathParts.pop();
  const repoName = (repo || '').split('/').pop();
  document.getElementById('kb-md-panel-title').textContent =
    pathParts.length > 0 ? `${repoName}/.../${fileName}` : `${repoName}/${fileName}`;
  document.getElementById('kb-md-github-link').href = url || '#';
  document.getElementById('kb-btn-copy-http').dataset.url = url || '';
  document.getElementById('kb-btn-copy-http').dataset.tip = url || '';
  const localPath = state.ui.knowledgeCorpusRoot ? `${state.ui.knowledgeCorpusRoot}/${repoName}/${path}` : `${repoName}/${path}`;
  document.getElementById('kb-btn-copy-path').dataset.path = localPath;
  document.getElementById('kb-btn-copy-path').dataset.tip = localPath;
  document.getElementById('kb-md-file-size').textContent = '';

  const itermBtn = document.getElementById('kb-btn-open-iterm');
  itermBtn.style.display = '';
  itermBtn.onclick = async () => {
    itermBtn.disabled = true;
    try {
      const res = await api.openItermAt(repo);
      if (res.error) alert(`Failed to open terminal: ${res.error}`);
    } catch (e) {
      alert(`Failed to open terminal: ${e.message}`);
    } finally {
      itermBtn.disabled = false;
    }
  };

  const tagsBar = document.getElementById('md-tags-bar');
  if (tagsBar) tagsBar.style.display = 'none';

  const modal = document.getElementById('kb-md-modal');
  const body = document.getElementById('kb-md-body');
  body.innerHTML = '<div style="color:#8c959f;padding:20px;font-size:13px;">Loading…</div>';
  modal.style.display = 'flex';
  document.body.style.overflow = 'hidden';

  try {
    const [mdResult, annResult] = await Promise.allSettled([
      api.fetchKbFileContent(repo, path),
      api.fetchKbAnnotation(repo, path)
    ]);

    const ann = annResult.status === 'fulfilled' ? (annResult.value || {}) : {};
    state.viewer.annotation = ann;

    if (mdResult.status === 'rejected') {
      throw new Error(mdResult.reason?.message || 'fetch failed');
    }
    const result = mdResult.value;
    if (result.error) throw new Error(result.error);

    const text = result.content;
    state.viewer.rawText = text;

    const bytes = new Blob([text]).size;
    document.getElementById('kb-md-file-size').textContent = bytes < 1024
      ? `${bytes} B`
      : bytes < 1024 * 1024
        ? `${(bytes / 1024).toFixed(1)} KB`
        : `${(bytes / (1024 * 1024)).toFixed(1)} MB`;

    if (typeof marked !== 'undefined') {
      await renderKbMdBody(text);
    } else {
      body.innerHTML = `<pre style="white-space:pre-wrap;font-size:13px">${escHtml(text)}</pre>`;
    }

    document.getElementById('kb-btn-edit').style.display = '';

    initKbComments();
    initKbHighlightUI();

    // Restore badge if there are already pending changes
    api.fetchKbStatus(repo).then(data => {
      if (!data.error && (data.total > 0 || data.ahead > 0)) {
        kbShowPendingBadge('chore: update via viewer');
      }
    }).catch(() => {});

    // Listen for dirty events from comments/highlights
    document.addEventListener('kb:dirty', onKbDirty);
  } catch (e) {
    body.innerHTML = `<div style="color:#7d4e00;padding:20px">Could not load file: ${escHtml(e.message)}</div>`;
    document.getElementById('kb-btn-edit').style.display = 'none';
  }
}

// ── saveKbDoc ──────────────────────────────────────────────────────────────
export async function saveKbDoc() {
  const editArea = document.getElementById('kb-md-edit-area');
  const newContent = editArea.value;
  const btnSave = document.getElementById('kb-btn-save');
  btnSave.disabled = true;
  btnSave.textContent = 'Saving…';
  try {
    const originalContent = state.viewer.rawText;
    const data = await api.saveKbFile(state.viewer.kbRepo, state.viewer.kbPath, newContent);
    if (data.error) throw new Error(data.error);
    state.viewer.rawText = newContent;
    _kbExitEditMode();

    const body = document.getElementById('kb-md-body');
    if (typeof marked !== 'undefined') {
      await renderKbMdBody(newContent);
    } else {
      body.innerHTML = `<pre style="white-space:pre-wrap;font-size:13px">${escHtml(newContent)}</pre>`;
    }
    document.getElementById('kb-btn-edit').style.display = '';
    if (newContent !== originalContent) {
      kbShowPendingBadge('update: edit via viewer');
      showKbReindexBtn(state.viewer.kbRepo);
    }
  } catch (e) {
    alert(`Save failed: ${e.message}`);
  } finally {
    btnSave.disabled = false;
    btnSave.textContent = '💾 Save';
  }
}
// ── closeKbModal ───────────────────────────────────────────────────────────
export function closeKbModal() {
  document.getElementById('kb-md-modal').style.display = 'none';
  document.removeEventListener('kb:dirty', onKbDirty);
  state.viewer.isKb = false;
  state.viewer.kbRepo = null;
  state.viewer.kbPath = null;
  state.viewer.annotation = {};
  document.body.style.overflow = '';
  kbHidePendingBadge();
}

export function _kbEnterEditMode() {
  const body = document.getElementById('kb-md-body');
  const editArea = document.getElementById('kb-md-edit-area');
  setDocEditMode({
    bodyEl: body,
    editAreaEl: editArea,
    text: state.viewer.rawText,
    editing: true,
  });
  document.getElementById('kb-btn-save').style.display = '';
  document.getElementById('kb-btn-cancel-edit').style.display = '';
  document.getElementById('kb-btn-edit').style.display = 'none';
  resetEditAreaScroll(editArea, { focus: true });
}

export function _kbExitEditMode() {
  const body = document.getElementById('kb-md-body');
  const editArea = document.getElementById('kb-md-edit-area');
  setDocEditMode({ bodyEl: body, editAreaEl: editArea, editing: false });
  document.getElementById('kb-btn-save').style.display = 'none';
  document.getElementById('kb-btn-cancel-edit').style.display = 'none';
  document.getElementById('kb-btn-edit').style.display = '';
}
