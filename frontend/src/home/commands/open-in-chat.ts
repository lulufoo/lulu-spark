import { navigate } from '../../router/index.ts';
import { cancelComposerDraft, requestComposerDraft } from './composer-draft.ts';
import { createSession } from './hub.ts';

/**
 * Start a fresh conversation with the composer prefilled; nothing is sent or staged.
 * The draft is requested before the session exists because the home composer
 * consumes it as soon as the new session id lands in state.
 */
export async function openDraftInChat(draft: string) {
  const text = String(draft || '');
  if (!text.trim()) throw new Error('Missing draft');
  requestComposerDraft(text);
  try {
    const ok = await createSession();
    if (!ok) {
      cancelComposerDraft();
      return;
    }
  } catch (err) {
    cancelComposerDraft();
    throw err;
  }
  navigate('#/home');
}
