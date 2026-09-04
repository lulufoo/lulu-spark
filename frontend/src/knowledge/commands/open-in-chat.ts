import { openPathInChat } from '../../home/commands/open-in-chat.ts';

export async function openKnowledgeInChat(path: string, button?: HTMLButtonElement | null) {
  if (button) button.disabled = true;
  try {
    await openPathInChat(path);
  } finally {
    if (button) button.disabled = false;
  }
}
