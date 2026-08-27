import { state } from '../../host/state.js';
import { resetEditAreaScroll } from '../../shared/utils.js';
import * as api from '../../host/api.js';
import { navigateToNote, navigateBackToList, parseHash } from '../../router/index.js';
import { closeCommitDialog } from './commit.js';
import { exitEditMode } from './doc.js';
import { hideNoteOutlet, setNotePanelTitle, showNoteOutlet } from './outlet.js';

const CREATE_CHROME_HIDDEN_IDS = [
  'btn-edit',
  'btn-add-comment',
  'btn-save',
  'btn-cancel-edit',
  'btn-panel-commit',
  'md-github-link',
  'md-lang-bar',
  'md-file-size',
  'md-links-bar',
  'md-tags-bar',
  'knowledge-panel',
  'btn-copy-http',
  'btn-copy-path',
  'btn-goto-kb',
  'btn-open-iterm',
  'comment-float-nav',
  'md-commit-bar',
];

/** @type {Record<string, string>|null} */
let createChromePrevDisplay = null;

export function applyCreateChrome() {
  const outlet = document.getElementById('note-outlet');
  if (outlet) outlet.classList.add('is-create');
  createChromePrevDisplay = {};
  for (const id of CREATE_CHROME_HIDDEN_IDS) {
    const el = document.getElementById(id);
    if (!el) continue;
    createChromePrevDisplay[id] = el.style.display;
    el.style.display = 'none';
  }
  setNotePanelTitle(state.ui.activeDate || '');
  document.getElementById('md-body').style.display = 'none';
  const editArea = document.getElementById('md-edit-area');
  editArea.style.display = '';
}

export function clearCreateChrome() {
  const outlet = document.getElementById('note-outlet');
  if (outlet) outlet.classList.remove('is-create');
  if (createChromePrevDisplay) {
    for (const [id, display] of Object.entries(createChromePrevDisplay)) {
      const el = document.getElementById(id);
      if (el) el.style.display = display;
    }
    createChromePrevDisplay = null;
  }
}

export function locationDate() {
  const route = parseHash();
  return route.params?.date || state.ui.activeDate || '';
}

export function dismissViewerModal() {
  clearCreateChrome();
  hideNoteOutlet();
  document.body.style.overflow = '';
  exitEditMode(false);
  closeCommitDialog();
  navigateBackToList({ date: locationDate() });
}

/**
 * Enter create session on the shared viewer (body-only chrome).
 * Binds crash buffer at drafts/notes/<temp_id>. Does not pretend an index entry exists.
 * @param {{ temp_id: string }} opts
 */
export async function openCreateNote({ temp_id } = {}) {
  if (!temp_id) return;
  const outlet = document.getElementById('note-outlet');
  const prevHidden = outlet ? outlet.hidden : true;
  const prevMode = outlet?.dataset?.wbMode || '';
  try {
    let content = '';
    try {
      const draft = await api.getNoteDraft(temp_id);
      content = typeof draft?.content === 'string' ? draft.content : '';
    } catch {
      content = '';
    }
    await api.saveNoteDraft(temp_id, content);

    state.viewer.entry = null;
    state.viewer.layer = 'raw';
    state.viewer.lang = null;
    state.viewer.annotation = {};
    state.viewer.rawText = content;
    state.viewer.isKb = false;
    state.viewer.createSession = { tempId: temp_id, status: 'creating' };

    closeCommitDialog();
    applyCreateChrome();

    const editArea = document.getElementById('md-edit-area');
    editArea.value = content;
    resetEditAreaScroll(editArea, { focus: true });

    // Create is outlet-local; location must omit note (no temp note id).
    showNoteOutlet('create');
  } catch (e) {
    state.viewer.createSession = null;
    clearCreateChrome();
    if (outlet) {
      outlet.hidden = prevHidden;
      outlet.dataset.wbMode = prevMode;
    }
    const msg = e instanceof Error ? e.message : String(e);
    alert(`Could not open new note: ${msg}`);
  }
}

export async function finalizeCreateSession() {
  const session = state.viewer.createSession;
  if (!session || session.status === 'saving') return;

  const editArea = document.getElementById('md-edit-area');
  const trimmed = (editArea?.value ?? '').trim();

  if (!trimmed) {
    await api.clearNoteDraft(session.tempId);
    state.viewer.createSession = null;
    dismissViewerModal();
    return;
  }

  session.status = 'saving';
  try {
    await api.saveNoteDraft(session.tempId, trimmed);
    const archived = await api.archiveDocument({ body: trimmed, source_type: 'note' });
    const commonPath = archived?.common_path;
    await api.clearNoteDraft(session.tempId);
    state.viewer.createSession = null;
    clearCreateChrome();
    const date = locationDate();
    if (commonPath && date) {
      navigateToNote({ date, note: commonPath });
    }
    showNoteOutlet('open');
  } catch (e) {
    session.status = 'creating';
    showNoteOutlet('create');
    alert(`Save failed: ${e.message}`);
  }
}

export async function closeModal() {
  if (state.viewer.createSession) {
    await finalizeCreateSession();
    return;
  }
  dismissViewerModal();
}
