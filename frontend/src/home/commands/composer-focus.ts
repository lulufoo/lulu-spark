let pendingComposerFocus = false;

export function requestComposerFocus() {
  pendingComposerFocus = true;
}

export function consumeComposerFocus() {
  const pending = pendingComposerFocus;
  pendingComposerFocus = false;
  return pending;
}
