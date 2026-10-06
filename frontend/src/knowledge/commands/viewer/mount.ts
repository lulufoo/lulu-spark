import { flushSync } from 'react-dom';
import { state } from '../../state/host.ts';
import { resetEditAreaScroll } from '../../../shared/utils.ts';
import * as api from '../../../host/api.ts';
import { initKbComments, cleanupKbComments } from '../../ui/comments.tsx';
import { setDocEditMode } from '../../../doc-editor/view.tsx';
import { cleanupKbHighlightUI, initKbHighlightUI } from '../../ui/viewer/highlight.ts';
import { revokeKbBlobUrls } from './images.ts';
import { paintKbMdBody } from './paint.ts';
import { openKnowledgeInChat } from '../open-in-chat.ts';
import { paintKbError, paintKbLoading, paintKbPlain, paintReaderShell } from '../../ui/viewer/shell.tsx';
import {
  errMessage,
  type KbReaderHost,
  type ReaderListener,
} from '../../state/types.ts';

let loadToken = 0;

export function bindReaderListener(
  listeners: ReaderListener[],
  el: EventTarget,
  type: string,
  handler: EventListener,
) {
  el.addEventListener(type, handler);
  listeners.push([el, type, handler]);
}

export function formatFileSize(text: string) {
  const bytes = new Blob([text]).size;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/**
 * @param {HTMLElement} container
 * @param {{ repo: string, path: string, url?: string }} opts
 * @returns {Promise<{ unmount: () => void }>}
 */
export async function mountKbReader(
  container: KbReaderHost,
  { repo, path }: { repo: string; path: string; url?: string },
) {
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
    fileSize: container.querySelector('.kb-file-size') as HTMLElement,
    btnCopyPath: container.querySelector('.kb-btn-copy-path') as HTMLButtonElement,
    btnOpenInChat: container.querySelector('.kb-btn-open-in-chat') as HTMLButtonElement,
    btnEdit: container.querySelector('.kb-btn-edit') as HTMLButtonElement,
    btnSave: container.querySelector('.kb-btn-save') as HTMLButtonElement,
    btnCancelEdit: container.querySelector('.kb-btn-cancel-edit') as HTMLButtonElement,
    btnAddComment: container.querySelector('.kb-btn-add-comment') as HTMLButtonElement,
    body: container.querySelector('.kb-reader-body') as HTMLElement,
    editArea: container.querySelector('.kb-reader-edit-area') as HTMLTextAreaElement,
  };

  const listeners: ReaderListener[] = [];

  const localPath = state.ui.knowledgeRoot
    ? `${state.ui.knowledgeRoot}/${(repo || '').split('/').pop()}/${path}`
    : `${(repo || '').split('/').pop()}/${path}`;
  ui.btnCopyPath.dataset.path = localPath;
  ui.btnCopyPath.dataset.tip = localPath;
  ui.fileSize.textContent = '';

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
      const data = await api.saveKbFile(state.viewer.kbRepo, state.viewer.kbPath, newContent);
      if (data.error) throw new Error(data.error);
      state.viewer.rawText = newContent;
      exitEditMode();
      if (typeof marked !== 'undefined') {
        await paintKbMdBody(newContent, ui.body);
      } else {
        paintKbPlain(ui.body, newContent);
      }
      ui.btnEdit.style.display = '';
    } catch (err) {
      const e = err as { message?: string };
      alert(`Save failed: ${e.message}`);
    } finally {
      ui.btnSave.disabled = false;
      ui.btnSave.textContent = 'Save';
    }
  }

  function unmount() {
    if (token !== loadToken) return;
    loadToken += 1;
    exitEditMode();
    cleanupKbComments();
    cleanupKbHighlightUI();
    revokeKbBlobUrls();
    for (const [el, type, handler] of listeners) {
      el.removeEventListener(type, handler);
    }
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
  }

  container._kbUnmount = unmount;

  bindReaderListener(listeners, ui.btnEdit, 'click', enterEditMode);
  bindReaderListener(listeners, ui.btnSave, 'click', () => {
    void saveDoc();
  });
  bindReaderListener(listeners, ui.btnCancelEdit, 'click', exitEditMode);
  bindReaderListener(listeners, ui.btnOpenInChat, 'click', () => {
    const copyPath = ui.btnCopyPath.dataset.path || '';
    void openKnowledgeInChat(copyPath, ui.btnOpenInChat).catch((err) => {
      alert(err instanceof Error ? err.message : String(err));
    });
  });
  bindReaderListener(listeners, ui.btnCopyPath, 'click', (e) => {
    const btn = e.currentTarget as HTMLButtonElement;
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
        throw new Error(errMessage(mdResult.reason, 'fetch failed'));
      }
      const result = mdResult.value as { error?: string; content?: string };
      if (result.error) throw new Error(result.error);

      const text = typeof result.content === 'string' ? result.content : '';
      state.viewer.rawText = text;
      ui.fileSize.textContent = formatFileSize(text);

      if (typeof marked !== 'undefined') {
        await paintKbMdBody(text, ui.body);
      } else {
        paintKbPlain(ui.body, text);
      }

      ui.btnEdit.style.display = '';
      const readerRoot = container.querySelector('.kb-reader') ?? container;
      initKbComments(readerRoot);
      initKbHighlightUI(readerRoot);
    } catch (e) {
      if (token !== loadToken) return;
      paintKbError(ui.body, errMessage(e, 'fetch failed'));
      ui.btnEdit.style.display = 'none';
    }
  })();

  return { unmount };
}
