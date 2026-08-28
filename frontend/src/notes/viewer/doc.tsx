// @ts-nocheck — ported from JS; state shapes stay unchecked like checkJs:false.
import { state, getEntryId, loadDiffStatus } from '../../host/state.ts';
import { getGithubUserUrl, workbenchGithubBlobBase } from '../../host/constants.ts';
import { getActivePath } from '../../corpus/corpus-path.ts';
import { filenameFromPath, slugToTitle, resetEditAreaScroll } from '../../shared/utils.ts';
import * as api from '../../host/api.ts';
import { updateTitlesInDOM, updateDiffInDOM } from '../cards.tsx';
import { setDocEditMode } from '../../doc-editor/view.tsx';
import { mountKnowledgeSearch, triggerKnowledgeSearch } from '../../corpus/corpus-knowledge-search.tsx';
import { saveKbDoc } from '../../corpus/corpus-viewer.ts';
import { closeCommitDialog, hidePendingBadge, showPendingBadge } from './commit.tsx';
import { initHighlightUI, renderDocBody } from './body.tsx';
import { setNotePanelTitle, showNoteOutlet } from './outlet.ts';
import { renderToHtml } from '../../island.ts';

function paintDocLoading(body: HTMLElement) {
  body.innerHTML = renderToHtml(
    <div style={{ color: '#8c959f', padding: 20, fontSize: 13 }}>Loading…</div>,
  );
}

function paintDocError(body: HTMLElement, message: string) {
  body.innerHTML = renderToHtml(
    <div style={{ color: '#7d4e00', padding: 20 }}>Could not load file: {message}</div>,
  );
}

export function updateLangBar(entry) {
  const bar = document.getElementById('md-lang-bar');
  const hasZh = !!entry.translations?.zh;
  bar.style.display = hasZh ? 'flex' : 'none';
  if (hasZh) {
    document.getElementById('btn-lang-en').classList.toggle('active', state.viewer.lang !== 'zh');
    document.getElementById('btn-lang-zh').classList.toggle('active', state.viewer.lang === 'zh');
  }
}

export function updateHeaderUrls(entry, layer, activePath) {
  const ghBase = workbenchGithubBlobBase(getGithubUserUrl(), state.ui.workbenchKnowledgeRoot);
  const githubUrl = ghBase ? `${ghBase}/${layer}/${activePath}` : '';
  const ghLink = document.getElementById('md-github-link');
  const copyHttp = document.getElementById('btn-copy-http');
  if (githubUrl) {
    ghLink.href = githubUrl;
    ghLink.style.display = '';
    copyHttp.dataset.url = githubUrl;
    copyHttp.dataset.tip = githubUrl;
    copyHttp.style.display = '';
  } else {
    ghLink.removeAttribute('href');
    ghLink.style.display = 'none';
    copyHttp.style.display = 'none';
  }
  const relPath = `${layer}/${activePath}`;
  const fullPath = state.ui.workbenchKnowledgeRoot ? `${state.ui.workbenchKnowledgeRoot}/${relPath}` : relPath;
  document.getElementById('btn-copy-path').dataset.tip = fullPath;

  const topicDir = entry.common_path.split('/')[0];
  const kbUrl = state.index.topicRepos[topicDir];
  const kbBtn = document.getElementById('btn-goto-kb');
  if (kbUrl) {
    kbBtn.style.display = '';
    kbBtn.onclick = () => window.open(kbUrl, '_blank', 'noopener,noreferrer');
  } else {
    kbBtn.style.display = 'none';
  }
}

// ── openDoc ────────────────────────────────────────────────────────────────

export async function openDoc(entry, layer = 'raw') {
  initHighlightUI();
  state.viewer.entry = entry;
  state.viewer.layer = layer;
  state.viewer.annotation = {};
  state.viewer.lang = entry.translations?.zh ? 'zh' : null;
  exitEditMode(false);
  closeCommitDialog();

  const body = document.getElementById('md-body');
  setNotePanelTitle(entry);
  const activePath = getActivePath(entry, state.viewer.lang, layer);
  updateHeaderUrls(entry, layer, activePath);
  updateLangBar(entry);
  document.getElementById('md-file-size').textContent = '';
  paintDocLoading(body);
  showNoteOutlet('open');

  const [mdResult, annResult] = await Promise.allSettled([
    api.fetchFileContent(layer, activePath),
    api.fetchAnnotation(entry.common_path).catch(() => ({}))
  ]);

  if (annResult.status === 'fulfilled') state.viewer.annotation = annResult.value || {};

  if (mdResult.status === 'rejected') {
    paintDocError(body, mdResult.reason.message);
    document.getElementById('btn-edit').style.display = 'none';
    return;
  }

  const text = mdResult.value;
  state.viewer.rawText = text;
  const bytes = new Blob([text]).size;
  document.getElementById('md-file-size').textContent = bytes < 1024
    ? `${bytes} B`
    : bytes < 1024 * 1024
      ? `${(bytes / 1024).toFixed(1)} KB`
      : `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  await renderDocBody(text, layer, activePath);
  const hasDiff = state.index.diffStatus.get(`${layer}/${entry.common_path}`);
  if (hasDiff) showPendingBadge(); else hidePendingBadge();

  mountKnowledgeSearch(document.getElementById('knowledge-panel'));
  triggerKnowledgeSearch(entry);
}

// ── switchLang ─────────────────────────────────────────────────────────────

export async function switchLang(lang) {
  if (!state.viewer.entry) return;
  const entry = state.viewer.entry;
  const layer = state.viewer.layer;

  // save current scroll position
  const body = document.getElementById('md-body');
  const cacheKey = `${getEntryId(entry)}:${layer}`;
  state.viewer.scrollCache[cacheKey] = body.scrollTop;

  state.viewer.lang = lang;
  const activePath = getActivePath(entry, lang, layer);
  updateHeaderUrls(entry, layer, activePath);
  updateLangBar(entry);

  paintDocLoading(body);

  try {
    const text = await api.fetchFileContent(layer, activePath);
    state.viewer.rawText = text;
    const bytes = new Blob([text]).size;
    document.getElementById('md-file-size').textContent = bytes < 1024
      ? `${bytes} B`
      : bytes < 1024 * 1024
        ? `${(bytes / 1024).toFixed(1)} KB`
        : `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    await renderDocBody(text, layer, activePath);
    // restore scroll position
    const cacheKey = `${getEntryId(entry)}:${layer}`;
    const saved = state.viewer.scrollCache[cacheKey];
    if (saved != null) requestAnimationFrame(() => { body.scrollTop = saved; });
  } catch (e) {
    paintDocError(body, e.message);
  }
}

// ── Edit mode ──────────────────────────────────────────────────────────────

export function enterEditMode() {
  const editArea = document.getElementById('md-edit-area');
  const body = document.getElementById('md-body');
  setDocEditMode({ bodyEl: body, editAreaEl: editArea, text: state.viewer.rawText, editing: true });
  resetEditAreaScroll(editArea, { focus: true });
  document.getElementById('btn-edit').style.display = 'none';
  document.getElementById('btn-add-comment').style.display = 'none';
  hidePendingBadge();
  document.getElementById('btn-save').style.display = '';
  document.getElementById('btn-cancel-edit').style.display = '';
  document.getElementById('md-github-link').style.display = 'none';
  closeCommitDialog();
}

export function exitEditMode(rerender = true) {
  const editArea = document.getElementById('md-edit-area');
  const body = document.getElementById('md-body');
  setDocEditMode({ bodyEl: body, editAreaEl: editArea, editing: false });
  document.getElementById('btn-edit').style.display = '';
  document.getElementById('btn-add-comment').style.display = '';
  document.getElementById('btn-save').style.display = 'none';
  document.getElementById('btn-cancel-edit').style.display = 'none';
  const _layer = state.viewer.layer || 'raw';
  const _cp = state.viewer.entry?.common_path;
  const _hasDiff = _cp ? state.index.diffStatus.get(`${_layer}/${_cp}`) : false;
  if (_hasDiff) showPendingBadge(); else hidePendingBadge();
  document.getElementById('md-github-link').style.display = '';
  if (rerender && state.viewer.entry) {
    const activePath = getActivePath(state.viewer.entry, state.viewer.lang, state.viewer.layer);
    void renderDocBody(state.viewer.rawText, state.viewer.layer, activePath);
  }
}

export async function saveDoc() {
  if (state.viewer.isKb) {
    await saveKbDoc();
    return;
  }
  if (!state.viewer.entry) return;
  const editArea = document.getElementById('md-edit-area');
  const newContent = editArea.value;
  const btnSave = document.getElementById('btn-save');
  btnSave.disabled = true;
  btnSave.textContent = 'Saving…';

  try {
    const activePath = getActivePath(state.viewer.entry, state.viewer.lang, state.viewer.layer);
    const data = await api.saveFile(state.viewer.layer, activePath, newContent);
    if (data.error) throw new Error(data.error);

    state.viewer.rawText = newContent;
    const date = state.viewer.entry.created_at.slice(0, 8);
    const h1Match = newContent.match(/^#\s+(.+)/m);
    const newTitle = h1Match ? h1Match[1].trim() : slugToTitle(filenameFromPath(state.viewer.entry.common_path));
    const entryId = getEntryId(state.viewer.entry);
    if (!state.index.titleCache.has(date)) state.index.titleCache.set(date, new Map());
    state.index.titleCache.get(date).set(entryId, newTitle);
    const card = entryId ? document.querySelector(`.doc-card[data-id="${entryId}"]`) : null;
    if (card) {
      const titleEl = card.querySelector('.doc-title-btn');
      if (titleEl) { titleEl.classList.remove('loading'); titleEl.textContent = newTitle; }
    } else {
      updateTitlesInDOM(date);
    }
    exitEditMode(true);
    await loadDiffStatus();
    updateDiffInDOM();
    showPendingBadge();
  } catch (e) {
    alert(`Save failed: ${e.message}`);
  } finally {
    btnSave.disabled = false;
    btnSave.textContent = '💾 Save';
  }
}
