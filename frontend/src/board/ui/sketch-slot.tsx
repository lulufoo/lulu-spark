import { createElement } from 'react';
import { flushSync } from 'react-dom';
import { createRoot, type Root } from 'react-dom/client';
import type { SketchAttachment } from '../state/sketch-attachment.ts';
import { SketchPreview } from './sketch-preview.tsx';
import { currentSketchAppearance, watchSketchTheme } from './sketch-theme.ts';

/** A fallback removes this slot; a later trusted source can create it again. */
export function paintSketchAttachment(attachment: SketchAttachment): () => void {
  const { anchor, pageUrl, view } = attachment;
  const wasHidden = anchor instanceof HTMLElement && anchor.hidden;
  let root: Root | undefined;
  let slot: HTMLElement | undefined;
  let note: HTMLElement | undefined;

  function removeSlot() {
    if (root) flushSync(() => root?.unmount());
    root = undefined;
    slot?.remove();
    slot = undefined;
    if (anchor instanceof HTMLElement) anchor.hidden = wasHidden;
  }

  function paint() {
    if (!attachment.isLive() || view.mode === 'link') {
      removeSlot();
      note?.remove();
      note = undefined;
      if (attachment.isLive() && view.otherFormat) {
        note = document.createElement('span');
        note.className = 'board-sketch-note';
        note.textContent = 'Other Board formats stay as links';
        anchor.insertAdjacentElement('afterend', note);
      }
      return;
    }
    note?.remove();
    note = undefined;
    if (!root) {
      slot = document.createElement('div');
      slot.className = 'board-sketch-slot';
      anchor.insertAdjacentElement('afterend', slot);
      root = createRoot(slot);
      if (anchor instanceof HTMLElement) anchor.hidden = true;
    }
    flushSync(() => root?.render(createElement(SketchPreview, {
      key: view.revision, html: view.html, pageUrl,
      appearance: currentSketchAppearance(), title: view.title,
    })));
  }

  const stopState = attachment.subscribe(paint);
  const stopTheme = watchSketchTheme(paint);
  paint();
  return () => {
    stopState();
    stopTheme();
    removeSlot();
    note?.remove();
  };
}
