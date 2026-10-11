import * as api from '../../host/api.ts';
import { openDraftInChat } from '../../home/commands/open-in-chat.ts';
import { serializeReference } from '../../home/references/index.ts';
import { formatReferenceText, writeClipboardText } from '../../shared/reference-text.ts';
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

/** Ask the host for this document's id, issuing one if it has none yet. */
async function rememberKnowledgeId(path: string) {
  const abs = String(path || '').trim();
  if (!abs) throw new Error('Missing file path');
  const remembered = (await api.invoke('remember_knowledge_doc', { path: abs })) as {
    ok?: boolean;
    id?: string;
  };
  const id = String(remembered?.id || '').trim();
  if (!id) throw new Error('Missing source');
  return { abs, id };
}

/** Copy `Lulu Spark knowledge` / title / id for an outside tool. Resolves false when nothing was copied. */
export async function copyKnowledgeReference(path: string) {
  const { abs, id } = await rememberKnowledgeId(path);
  const title = documentTitle(abs, viewingKnowledgeBody(abs));
  return writeClipboardText(formatReferenceText({ kind: 'knowledge', id, title }));
}

export async function openKnowledgeInChat(path: string, button?: HTMLButtonElement | null) {
  if (button) button.disabled = true;
  try {
    const { abs, id: sourceId } = await rememberKnowledgeId(path);
    await openDraftInChat(knowledgeReferenceDraft(abs, sourceId, viewingKnowledgeBody(abs)));
  } finally {
    if (button) button.disabled = false;
  }
}
