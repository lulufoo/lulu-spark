import * as api from '../../host/api.ts';
import { openDraftInChat } from '../../home/commands/open-in-chat.ts';
import { serializeReference } from '../../home/references/index.ts';
import { state } from '../state/host.ts';

function filenameTitle(path: string) {
  const name = path.replace(/\s+/g, ' ').trim().split('/').pop() ?? '';
  return name.replace(/\.[^.]*$/, '') || name;
}

function h1Title(text: string) {
  return String(text || '').match(/^#\s+(.+)/m)?.[1]?.trim() || '';
}

function viewingKnowledgeBody(path: string) {
  if (!state.viewer.isKb) return '';
  const kbPath = String(state.viewer.kbPath || '').replace(/\s+/g, ' ').trim();
  const abs = String(path || '').replace(/\s+/g, ' ').trim();
  if (!kbPath || !abs) return '';
  if (abs !== kbPath && !abs.endsWith(`/${kbPath}`)) return '';
  return state.viewer.rawText;
}

function documentTitle(path: string, body = '') {
  return h1Title(body) || filenameTitle(path);
}

/** `[title](knowledge:<id>) ` — shown as a chip in the composer; the space keeps typing after it. */
export function knowledgeReferenceDraft(path: string, id: string, body = '') {
  return `${serializeReference({ kind: 'knowledge', id, title: documentTitle(path, body) })} `;
}

export async function openKnowledgeInChat(path: string, button?: HTMLButtonElement | null) {
  const abs = String(path || '').trim();
  if (!abs) throw new Error('Missing file path');
  if (button) button.disabled = true;
  try {
    const remembered = (await api.invoke('remember_knowledge_doc', { path: abs })) as {
      ok?: boolean;
      id?: string;
    };
    const sourceId = String(remembered?.id || '').trim();
    if (!sourceId) throw new Error('Missing source');
    await openDraftInChat(knowledgeReferenceDraft(abs, sourceId, viewingKnowledgeBody(abs)));
  } finally {
    if (button) button.disabled = false;
  }
}
