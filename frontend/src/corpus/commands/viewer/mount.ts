// @ts-nocheck — ported from JS; container._kbUnmount and viewer state stay unchecked.
import { flushSync } from 'react-dom';
import { state } from '../../state/host.ts';
import { resetEditAreaScroll } from '../../../shared/utils.ts';
import * as api from '../../../host/api.ts';
import { initKbComments, cleanupKbComments } from '../../ui/comments.tsx';
import { setDocEditMode } from '../../../doc-editor/view.tsx';
import { openKbCommitDialog } from './commit.ts';
import { cleanupKbHighlightUI, initKbHighlightUI, renderKbMdBody } from '../../ui/viewer/highlight.ts';
import { paintKbError, paintKbLoading, paintKbPlain, paintReaderShell } from '../../ui/viewer/shell.tsx';

let loadToken = 0;

export function bindReaderListener(listeners, el, type, handler) {
  el.addEventListener(type, handler);
  listeners.push([el, type, handler]);
}

export function formatFileSize(text) {
  const bytes = new Blob([text]).size;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function buildReaderTitle(repo, path) {
  const pathParts = (path || '').split('/');
  const fileName = pathParts.pop();
  const repoName = (repo || '').split('/').pop();
  return pathParts.length > 0 ? `${repoName}/.../${fileName}` : `${repoName}/${fileName}`;
}

export function wireReindexBtn(btn, repo) {
  btn.textContent = '↺ Rebuild index';
  btn.title = 'Rebuild search index for this library';
  btn.disabled = false;
  btn.style.display = '';
  btn.onclick = async () => {
    btn.disabled = true;
    btn.textContent = 'Rebuilding…';
    try {
      const res = await api.reindexKbRepo(repo);
      if (res.error) throw new Error(res.error);
      const poll = setInterval(async () => {
        try {
          const status = await api.getReindexStatus();
          if (status.status === 'done') {
            clearInterval(poll);
            btn.textContent = '✓ Rebuilt';
            btn.disabled = false;
            setTimeout(() => {
              btn.style.display = 'none';
            }, 2000);
          } else if (status.status === 'error') {
            clearInterval(poll);
            btn.textContent = 'Rebuild failed';
            btn.disabled = false;
            btn.title = status.log || 'Unknown error';
          }
        } catch {
          // keep polling
        }
      }, 2000);
    } catch (e) {
      btn.textContent = 'Rebuild failed';
      btn.disabled = false;
      btn.title = e.message;
    }
  };
}

/**
 * @param {HTMLElement} container
 * @param {{ repo: string, path: string, url?: string }} opts
 * @returns {Promise<{ unmount: () => void }>}
 */
export async function mountKbReader(container, { repo, path, url }) {
  if (container._kbUnmount) {
    container._kbUnmount();
  }

  loadToken += 1;
  const token = loadToken;

  state.viewer.entry = null;
  state.viewer.isKb = true;
  state.viewer.kbRepo = repo;
  state.viewer.kbPath = path;
  state.viewer.layer = 'raw';
  state.viewer.rawText = '';
  state.viewer.annotation = {};
  state.viewer.lang = null;

  let root = paintReaderShell(container);

  const ui = {
    title: container.querySelector('.kb-reader-title'),
    fileSize: container.querySelector('.kb-file-size'),
    githubLink: container.querySelector('.kb-github-link'),
    itermBtn: container.querySelector('.kb-btn-open-iterm'),
    btnCopyHttp: container.querySelector('.kb-btn-copy-http'),
    btnCopyPath: container.querySelector('.kb-btn-copy-path'),
    btnEdit: container.querySelector('.kb-btn-edit'),
    btnSave: container.querySelector('.kb-btn-save'),
    btnCancelEdit: container.querySelector('.kb-btn-cancel-edit'),
    btnAddComment: container.querySelector('.kb-btn-add-comment'),
    btnPending: container.querySelector('.kb-btn-pending'),
    btnReindex: container.querySelector('.kb-btn-reindex'),
    body: container.querySelector('.kb-reader-body'),
    editArea: container.querySelector('.kb-reader-edit-area'),
  };

  /** @type {Array<[HTMLElement, string, (...args: any[]) => void]>} */
  const listeners = [];
  let pendingMsg = '';

  ui.title.textContent = buildReaderTitle(repo, path);
  ui.githubLink.href = url || '#';
  ui.btnCopyHttp.dataset.url = url || '';
  ui.btnCopyHttp.dataset.tip = url || '';
  const localPath = state.ui.knowledgeCorpusRoot
    ? `${state.ui.knowledgeCorpusRoot}/${(repo || '').split('/').pop()}/${path}`
    : `${(repo || '').split('/').pop()}/${path}`;
  ui.btnCopyPath.dataset.path = localPath;
  ui.btnCopyPath.dataset.tip = localPath;
  ui.fileSize.textContent = '';
  ui.itermBtn.style.display = '';
  wireReindexBtn(ui.btnReindex, repo);

  function showPendingBadge(msg) {
    pendingMsg = msg;
    ui.btnPending.style.display = '';
  }

  function hidePendingBadge() {
    pendingMsg = '';
    ui.btnPending.style.display = 'none';
  }

  function enterEditMode() {
    setDocEditMode({
      bodyEl: ui.body,
      editAreaEl: ui.editArea,
      text: state.viewer.rawText,
      editing: true,
    });
    ui.btnSave.style.display = '';
    ui.btnCancelEdit.style.display = '';
    ui.btnEdit.style.display = 'none';
    resetEditAreaScroll(ui.editArea, { focus: true });
  }

  function exitEditMode() {
    setDocEditMode({ bodyEl: ui.body, editAreaEl: ui.editArea, editing: false });
    ui.btnSave.style.display = 'none';
    ui.btnCancelEdit.style.display = 'none';
    ui.btnEdit.style.display = '';
  }

  async function saveDoc() {
    const newContent = ui.editArea.value;
    ui.btnSave.disabled = true;
    ui.btnSave.textContent = 'Saving…';
    try {
      const originalContent = state.viewer.rawText;
      const data = await api.saveKbFile(state.viewer.kbRepo, state.viewer.kbPath, newContent);
      if (data.error) throw new Error(data.error);
      state.viewer.rawText = newContent;
      exitEditMode();
      if (typeof marked !== 'undefined') {
        await renderKbMdBody(newContent, ui.body);
      } else {
        paintKbPlain(ui.body, newContent);
      }
      ui.btnEdit.style.display = '';
      if (newContent !== originalContent) {
        showPendingBadge('update: edit via viewer');
        wireReindexBtn(ui.btnReindex, state.viewer.kbRepo);
      }
    } catch (e) {
      alert(`Save failed: ${e.message}`);
    } finally {
      ui.btnSave.disabled = false;
      ui.btnSave.textContent = '💾 Save';
    }
  }

  function onDirty(e) {
    showPendingBadge(e.detail?.msg || 'chore: update via viewer');
  }

  function unmount() {
    if (token !== loadToken) return;
    loadToken += 1;
    exitEditMode();
    cleanupKbComments();
    cleanupKbHighlightUI();
    for (const [el, type, handler] of listeners) {
      el.removeEventListener(type, handler);
    }
    document.removeEventListener('kb:dirty', onDirty);
    if (root) {
      flushSync(() => {
        root?.unmount();
      });
      root = null;
      container.innerHTML = '';
    }
    delete container._kbUnmount;
    state.viewer.isKb = false;
    state.viewer.kbRepo = null;
    state.viewer.kbPath = null;
    state.viewer.annotation = {};
    hidePendingBadge();
  }

  container._kbUnmount = unmount;

  bindReaderListener(listeners, ui.btnEdit, 'click', enterEditMode);
  bindReaderListener(listeners, ui.btnSave, 'click', () => {
    void saveDoc();
  });
  bindReaderListener(listeners, ui.btnCancelEdit, 'click', exitEditMode);
  bindReaderListener(listeners, ui.btnPending, 'click', openKbCommitDialog);
  bindReaderListener(listeners, ui.btnCopyHttp, 'click', (e) => {
    const btn = e.currentTarget;
    const copyUrl = btn.dataset.url || '';
    if (!copyUrl) return;
    navigator.clipboard
      .writeText(copyUrl)
      .then(() => {
        const orig = btn.textContent;
        btn.textContent = '✓';
        setTimeout(() => {
          btn.textContent = orig;
        }, 1200);
      })
      .catch(() => {});
  });
  bindReaderListener(listeners, ui.btnCopyPath, 'click', (e) => {
    const btn = e.currentTarget;
    const copyPath = btn.dataset.path || '';
    if (!copyPath) return;
    navigator.clipboard
      .writeText(copyPath)
      .then(() => {
        const orig = btn.textContent;
        btn.textContent = '✓';
        setTimeout(() => {
          btn.textContent = orig;
        }, 1200);
      })
      .catch(() => {});
  });
  bindReaderListener(listeners, ui.itermBtn, 'click', async () => {
    ui.itermBtn.disabled = true;
    try {
      const res = await api.openItermAt(repo);
      if (res.error) alert(`Failed to open terminal: ${res.error}`);
    } catch (e) {
      alert(`Failed to open terminal: ${e.message}`);
    } finally {
      ui.itermBtn.disabled = false;
    }
  });

  document.addEventListener('kb:dirty', onDirty);

  paintKbLoading(ui.body);

  void (async () => {
    try {
      const [mdResult, annResult] = await Promise.allSettled([
        api.fetchKbFileContent(repo, path),
        api.fetchKbAnnotation(repo, path),
      ]);

      if (token !== loadToken) return;

      const ann = annResult.status === 'fulfilled' ? annResult.value || {} : {};
      state.viewer.annotation = ann;

      if (mdResult.status === 'rejected') {
        throw new Error(mdResult.reason?.message || 'fetch failed');
      }
      const result = mdResult.value;
      if (result.error) throw new Error(result.error);

      const text = result.content;
      state.viewer.rawText = text;
      ui.fileSize.textContent = formatFileSize(text);

      if (typeof marked !== 'undefined') {
        await renderKbMdBody(text, ui.body);
      } else {
        paintKbPlain(ui.body, text);
      }

      ui.btnEdit.style.display = '';
      const readerRoot = container.querySelector('.kb-reader') ?? container;
      initKbComments(readerRoot);
      initKbHighlightUI(readerRoot);

      api
        .fetchKbStatus(repo)
        .then((data) => {
          if (token !== loadToken) return;
          if (!data.error && (data.total > 0 || data.ahead > 0)) {
            showPendingBadge('chore: update via viewer');
          }
        })
        .catch(() => {});
    } catch (e) {
      if (token !== loadToken) return;
      paintKbError(ui.body, e.message);
      ui.btnEdit.style.display = 'none';
    }
  })();

  return { unmount };
}
