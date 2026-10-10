import { consumeComposerFocus, requestComposerFocus } from './composer-focus.ts';
import type { ComposerInputHandle } from '../ui/composer-input.tsx';

let pendingComposerDraft: string | null = null;

/** One-shot: prefill the home composer the next time it takes the requested focus. */
export function requestComposerDraft(text: string) {
  pendingComposerDraft = text;
  requestComposerFocus();
}

/** Drop a request that will never be consumed (e.g. session creation failed). */
export function cancelComposerDraft() {
  pendingComposerDraft = null;
  consumeComposerFocus();
}

export function consumeComposerDraft(): string | null {
  const draft = pendingComposerDraft;
  pendingComposerDraft = null;
  return draft;
}

/** Write the pending draft into the composer, caret at the end. No-op without a draft. */
export function applyComposerDraft(input: Pick<ComposerInputHandle, 'setText'> | null) {
  const draft = consumeComposerDraft();
  if (draft === null || !input) return;
  input.setText(draft);
}
