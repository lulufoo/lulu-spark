import { notifyState, state } from '../../state/host.ts';
import { resetEditAreaScroll } from '../../../shared/utils.ts';
import * as api from '../../../host/api.ts';
import { navigateToNote, navigateBackToList, parseHash } from '../../../router/index.ts';
import { closeCommitDialog } from './commit.ts';
import { exitEditMode } from './doc.ts';
import { hideNoteOutlet, setNotePanelTitle, showNoteOutlet } from './outlet.ts';

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

let createChromePrevDisplay: Record<string, string> | null = null;

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
  const body = document.getElementById('md-body');
  if (body) body.style.display = 'none';
  const editArea = document.getElementById('md-edit-area');
  if (editArea) editArea.style.display = '';
  state.viewer.outletMode = 'create';
  notifyState();
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
  if (state.viewer.outletMode === 'create' && !state.viewer.createSession) {
    state.viewer.outletMode = '';
  }
  notifyState();
}

export function locationDate() {
  const route = parseHash(typeof window !== 'undefined' ? window.location.hash : '');
  const params = route.params as { date?: string };
  return params.date || state.ui.activeDate || '';
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
export async function openCreateNote({ temp_id }: { temp_id?: string } = {}) {
  if (!temp_id) return;
  const prevMode = state.viewer.outletMode || '';
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
    state.viewer.editing = true;
    state.viewer.createSession = { tempId: temp_id, status: 'creating' };
    state.viewer.bodyPaintKey += 1;

    closeCommitDialog();
    applyCreateChrome();

    const editArea = document.getElementById('md-edit-area');
    if (editArea && 'value' in editArea) {
      const area = editArea as HTMLTextAreaElement;
      area.value = content;
      resetEditAreaScroll(area, { focus: true });
    }

    showNoteOutlet('create');
  } catch (e) {
    state.viewer.createSession = null;
    state.viewer.editing = false;
    state.viewer.outletMode = prevMode;
    clearCreateChrome();
    const msg = e instanceof Error ? e.message : String(e);
    alert(`Could not open new note: ${msg}`);
  }
}

export async function finalizeCreateSession() {
  const session = state.viewer.createSession;
  if (!session || session.status === 'saving') return;

  const editArea = document.getElementById('md-edit-area');
  const trimmed = (editArea && 'value' in editArea ? String((editArea as HTMLTextAreaElement).value) : '').trim();

  if (!trimmed) {
    await api.clearNoteDraft(session.tempId);
    state.viewer.createSession = null;
    dismissViewerModal();
    return;
  }

  session.status = 'saving';
  try {
    await api.saveNoteDraft(session.tempId, trimmed);
    const archived = (await api.createNote({ body: trimmed, source_type: 'jot' })) as {
      common_path?: string;
    };
    const commonPath = archived?.common_path;
    await api.clearNoteDraft(session.tempId);
    state.viewer.createSession = null;
    clearCreateChrome();
    const date = locationDate();
    if (commonPath && date) {
      navigateToNote({ date, note: commonPath });
    }
    showNoteOutlet('open');
  } catch (err) {
    session.status = 'creating';
    showNoteOutlet('create');
    const e = err as { message?: string };
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
