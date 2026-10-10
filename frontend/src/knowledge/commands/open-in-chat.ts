import * as api from '../../host/api.ts';
import { openDraftInChat } from '../../home/commands/open-in-chat.ts';

function documentTitle(path: string) {
  const name = path.replace(/\s+/g, ' ').trim().split('/').pop() ?? '';
  return name.replace(/\.[^.]*$/, '') || name;
}

/** Plain text, no link scheme: the model fetches the document with `get_knowledge_file`. */
export function knowledgeReferenceDraft(path: string, id: string) {
  return `请阅读知识库文档「${documentTitle(path)}」（id: ${id}）`;
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
    await openDraftInChat(knowledgeReferenceDraft(abs, sourceId));
  } finally {
    if (button) button.disabled = false;
  }
}
