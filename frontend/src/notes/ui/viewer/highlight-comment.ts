import {
  highlightCommentFragment,
  placeHighlightCommentPopover,
} from '../../../shared/tooltip/highlight-comment-popover.ts';

type HighlightCommentEditorOptions = {
  anchor: HTMLElement;
  point?: { x: number; y: number };
  text: string;
  onSave: (text: string) => Promise<void>;
};

let activeEditor: { anchor: HTMLElement; dismiss: (restoreFocus: boolean) => void } | null = null;

export function closeHighlightCommentEditor(): void {
  activeEditor?.dismiss(false);
}

function isSameHighlight(first: HTMLElement, second: HTMLElement) {
  return first === second || Boolean(first.dataset.hid && first.dataset.hid === second.dataset.hid);
}

export function openHighlightCommentEditor({
  anchor,
  point,
  text,
  onSave,
}: HighlightCommentEditorOptions): void {
  if (!anchor.isConnected) return;
  if (activeEditor?.anchor.isConnected && isSameHighlight(activeEditor.anchor, anchor)) return;
  closeHighlightCommentEditor();

  const fragment = highlightCommentFragment(anchor, point);
  const panel = document.createElement('div');
  panel.className = 'highlight-comment-popover highlight-comment-editor';
  panel.setAttribute('role', 'dialog');
  panel.setAttribute('aria-labelledby', 'highlight-comment-editor-title');
  panel.innerHTML = `
    <form class="highlight-comment-editor__form">
      <h3 id="highlight-comment-editor-title">Highlight comment</h3>
      <textarea aria-label="Highlight comment"></textarea>
      <p class="highlight-comment-editor__error" role="alert" hidden></p>
      <div class="highlight-comment-editor__actions">
        <button type="button" class="md-header-btn">Cancel</button>
        <button type="submit" class="md-header-btn primary">Save</button>
      </div>
    </form>
  `;
  const form = panel.querySelector('form')!;
  const input = panel.querySelector('textarea')!;
  const error = panel.querySelector<HTMLElement>('[role="alert"]')!;
  const cancel = panel.querySelector<HTMLButtonElement>('[type="button"]')!;
  const save = panel.querySelector<HTMLButtonElement>('[type="submit"]')!;
  const listeners = new window.AbortController();
  const { signal } = listeners;
  const observer = new MutationObserver(() => {
    if (!anchor.isConnected || !panel.isConnected) dismiss(false);
  });

  function dismiss(restoreFocus: boolean) {
    if (activeEditor?.dismiss !== dismiss) return;
    activeEditor = null;
    observer.disconnect();
    listeners.abort();
    panel.remove();
    anchor.setAttribute('aria-expanded', 'false');
    if (restoreFocus && anchor.isConnected) anchor.focus({ preventScroll: true });
  }

  function reposition() {
    if (!anchor.isConnected) {
      dismiss(false);
      return;
    }
    placeHighlightCommentPopover(panel, anchor, fragment);
  }

  activeEditor = { anchor, dismiss };
  input.value = text;
  document.body.appendChild(panel);
  anchor.setAttribute('aria-expanded', 'true');
  observer.observe(document.body, { childList: true, subtree: true });
  window.addEventListener('resize', reposition, { signal });
  window.addEventListener('scroll', reposition, { capture: true, signal });
  document.addEventListener('pointerdown', (event) => {
    const target = event.target;
    if (save.disabled || !(target instanceof Node) || panel.contains(target) || anchor.contains(target)) return;
    const otherMark = target instanceof Element ? target.closest<HTMLElement>('mark.doc-highlight.is-commentable') : null;
    if (otherMark && isSameHighlight(anchor, otherMark)) return;
    dismiss(false);
  }, { capture: true, signal });
  document.addEventListener('keydown', (event) => {
    if (event.key !== 'Escape' || event.isComposing) return;
    event.preventDefault();
    event.stopPropagation();
    if (!save.disabled) dismiss(true);
  }, { capture: true, signal });
  cancel.addEventListener('click', () => dismiss(true), { signal });
  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    if (save.disabled) return;
    error.hidden = true;
    input.readOnly = true;
    cancel.disabled = true;
    save.disabled = true;
    panel.setAttribute('aria-busy', 'true');
    try {
      await onSave(input.value);
      dismiss(true);
    } catch (cause) {
      if (!panel.isConnected) return;
      error.textContent = cause instanceof Error && cause.message
        ? cause.message
        : 'Could not save the highlight comment.';
      error.hidden = false;
    } finally {
      input.readOnly = false;
      cancel.disabled = false;
      save.disabled = false;
      panel.removeAttribute('aria-busy');
      if (panel.isConnected) {
        reposition();
        input.focus({ preventScroll: true });
      }
    }
  }, { signal });
  reposition();
  input.focus({ preventScroll: true });
  input.setSelectionRange(input.value.length, input.value.length);
}
