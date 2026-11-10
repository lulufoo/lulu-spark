import { releaseBoardSketches } from '../../../board/commands/sketch-links.ts';
import { state } from '../../state/host.ts';
import { resetEditAreaScroll } from '../../../shared/utils.ts';
import * as api from '../../../host/api.ts';
import { navigateToNote, navigateBackToList, parseHash } from '../../../router/index.ts';
import { exitEditMode } from './doc.ts';
import { forgetOpenDoc } from '../viewer-state.ts';
import { refreshNotesIndex } from '../reload-index.ts';
import { landOnNewDocument } from '../land-on-document.ts';
import { applyCreateChrome, clearCreateChrome, hideNoteOutlet, showNoteOutlet } from './outlet.ts';

export { applyCreateChrome, clearCreateChrome };

export function locationDate() {
  const route = parseHash(typeof window !== 'undefined' ? window.location.hash : '');
  const params = route.params as { date?: string };
  return params.date || state.ui.activeDate || '';
}

export function dismissViewerModal() {
  releaseBoardSketches(document.getElementById('md-body'));
  forgetOpenDoc();
  clearCreateChrome();
  hideNoteOutlet();
  document.body.style.overflow = '';
  exitEditMode(false);
  navigateBackToList({ date: locationDate() });
}

/**
 * Enter create session on the shared viewer (body-only chrome).
 * Binds crash buffer at drafts/docs/<temp_id>. Does not pretend an index entry exists.
 * @param {{ temp_id: string }} opts
 */
export async function openCreateNote(
  {
    temp_id,
    workspace_id,
    project_id,
    folder,
  }: { temp_id?: string; workspace_id?: string; project_id?: string; folder?: string } = {},
) {
  if (!temp_id) return;
  const prevMode = state.viewer.outletMode || '';
  try {
    let content = '';
    try {
      const draft = await api.getDocDraft(temp_id);
      content = typeof draft?.content === 'string' ? draft.content : '';
    } catch {
      content = '';
    }
    await api.saveDocDraft(temp_id, content);

    state.viewer.entry = null;
    state.viewer.layer = 'raw';
    state.viewer.lang = null;
    state.viewer.annotation = {};
    state.viewer.rawText = content;
    state.viewer.editing = true;
    state.viewer.createSession = {
      tempId: temp_id,
      status: 'creating',
      ...(workspace_id ? { workspace_id } : {}),
      ...(project_id ? { project_id } : {}),
      ...(folder ? { folder } : {}),
    };
    state.viewer.bodyPaintKey += 1;

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
    await api.clearDocDraft(session.tempId);
    state.viewer.createSession = null;
    dismissViewerModal();
    return;
  }

  session.status = 'saving';
  try {
    await api.saveDocDraft(session.tempId, trimmed);
    const workspaceId = session.workspace_id || state.workspaces.current;
    const archived = (await api.createNote({
      body: trimmed,
      source_type: 'jot',
      workspace_id: workspaceId,
      ...(session.project_id ? { project_id: session.project_id } : {}),
      ...(session.folder ? { folder: session.folder } : {}),
    })) as { id?: string };
    const id = archived?.id;
    await api.clearDocDraft(session.tempId);
    state.viewer.createSession = null;
    clearCreateChrome();
    let date = '';
    if (id && workspaceId === state.workspaces.current) {
      // The new document is not in the page's index yet; read it, then land where it lives.
      await refreshNotesIndex();
      date = landOnNewDocument(id, session.project_id || '');
    }
    date = date || locationDate();
    if (id && date) {
      navigateToNote({ date, note: id });
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
