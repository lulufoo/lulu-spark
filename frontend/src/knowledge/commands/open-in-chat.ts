import { openDraftInChat } from '../../home/commands/open-in-chat.ts';
import { serializeReference } from '../../home/references/index.ts';
import { formatReferenceText, writeClipboardText } from '../../shared/reference-text.ts';
import { state } from '../state/host.ts';

function h1Title(text: string) {
  return String(text || '').match(/^#\s+(.+)/m)?.[1]?.trim() || '';
}

function viewingKnowledgeBody(id: string) {
  if (!state.viewer.isKb) return '';
  const kbId = String(state.viewer.kbId || '').trim();
  if (!kbId || kbId !== id) return '';
  return state.viewer.rawText;
}

function documentTitle(body = '', titleHint = '') {
  return h1Title(body) || titleHint || 'knowledge';
}

/** `[title](knowledge:<id>) ` — shown as a chip in the composer; the space keeps typing after it. */
export function knowledgeReferenceDraft(id: string, body = '', titleHint = '') {
  return `${serializeReference({ kind: 'knowledge', id, title: documentTitle(body, titleHint) })} `;
}

/** Copy `Lulu Spark knowledge` / title / id for an outside tool. Resolves false when nothing was copied. */
export async function copyKnowledgeReference(id: string) {
  const sourceId = String(id || '').trim();
  if (!sourceId) throw new Error('Missing source');
  const title = documentTitle(viewingKnowledgeBody(sourceId));
  return writeClipboardText(formatReferenceText({ kind: 'knowledge', id: sourceId, title }));
}

export async function openKnowledgeInChat(id: string, button?: HTMLButtonElement | null) {
  if (button) button.disabled = true;
  try {
    const sourceId = String(id || '').trim();
    if (!sourceId) throw new Error('Missing source');
    await openDraftInChat(knowledgeReferenceDraft(sourceId, viewingKnowledgeBody(sourceId)));
  } finally {
    if (button) button.disabled = false;
  }
}
