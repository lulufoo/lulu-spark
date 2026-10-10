let pendingComposerFocus = false;

export function requestComposerFocus() {
  pendingComposerFocus = true;
}

export function consumeComposerFocus() {
  const pending = pendingComposerFocus;
  pendingComposerFocus = false;
  return pending;
}

export function canFocusComposerFromDockTarget(
  input: EventTarget | null,
  target: EventTarget | null,
) {
  if (!(input instanceof Node) || !(target instanceof Node)) return false;
  if (input.contains(target)) return false;
  if (target instanceof Element && target.closest('button')) return false;
  return true;
}

export function focusComposerFromDock(
  input: { el: HTMLElement; focus(): void } | null,
  target: EventTarget | null,
  locked: boolean,
) {
  if (locked || !input) return false;
  if (!canFocusComposerFromDockTarget(input.el, target)) return false;
  input.focus();
  return true;
}
