import * as api from '../../host/api.ts';
import { openPathInChat } from '../../home/commands/open-in-chat.ts';

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
    await openPathInChat(abs, { kind: 'knowledge', id: sourceId });
  } finally {
    if (button) button.disabled = false;
  }
}
