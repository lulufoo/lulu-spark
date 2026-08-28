import * as api from '../host/api.ts';
import { nowTs } from '../shared/utils.ts';
import {
  wrapNthMatch,
  getOccurrenceIndex,
} from './highlight-utils.ts';
import type {
  ApplyHighlightsArgs,
  DeleteHighlightArgs,
  HighlightOverlayConfig,
  HighlightRecord,
  HighlightsPayload,
} from './types.ts';

const _cleanups: Array<() => void> = [];
let _btn: HTMLButtonElement | null = null;

function hideBtn() {
  if (_btn) _btn.style.display = 'none';
}

function resolveBtn(buttonId?: string): HTMLButtonElement {
  if (buttonId) {
    const existing = document.getElementById(buttonId);
    if (existing instanceof HTMLButtonElement) return existing;
  }
  let btn = document.getElementById('doc-highlight-add-btn') as HTMLButtonElement | null;
  if (!btn) {
    btn = document.createElement('button');
    btn.id = 'doc-highlight-add-btn';
    btn.className = 'viewer-highlight-btn highlight-add-btn';
    btn.style.cssText = 'display:none;position:fixed;z-index:9999;';
    btn.textContent = 'Highlight';
    document.body.appendChild(btn);
  }
  return btn;
}

function unwrapMarks(container: Element | null) {
  if (!container) return;
  container.querySelectorAll('mark.doc-highlight').forEach((mark) => {
    mark.querySelectorAll('.highlight-del-btn').forEach((b) => b.remove());
    mark.replaceWith(...Array.from(mark.childNodes));
  });
  container.normalize();
}

export async function applyCachedHighlights({ bodyEl, identityKey, excludeBarId = '' }: ApplyHighlightsArgs) {
  if (!bodyEl || !identityKey) return;
  unwrapMarks(bodyEl);
  let highlights: HighlightRecord[] = [];
  try {
    const data = (await api.fetchDocHighlights(identityKey)) as HighlightsPayload;
    highlights = Array.isArray(data?.highlights) ? data.highlights : [];
  } catch {
    highlights = [];
  }
  for (const h of highlights) {
    wrapNthMatch(
      bodyEl,
      h.text,
      h.occurrence ?? 0,
      h.id,
      (id) => {
        void deleteCachedHighlight({ bodyEl, identityKey, excludeBarId, id });
      },
      excludeBarId,
    );
  }
}

async function deleteCachedHighlight({ bodyEl, identityKey, excludeBarId, id }: DeleteHighlightArgs) {
  if (!identityKey || !id) return;
  try {
    const data = (await api.updateDocHighlights(identityKey, { id }, nowTs())) as HighlightsPayload;
    if (!data.ok) throw new Error(data.error || 'failed');
    await applyCachedHighlights({ bodyEl, identityKey, excludeBarId });
  } catch (e) {
    // @ts-expect-error overlay alert uses e.message
    alert(`Failed to remove highlight: ${e.message}`);
  }
}

export function cleanupDocHighlightOverlay() {
  for (const fn of _cleanups) fn();
  _cleanups.length = 0;
  hideBtn();
  _btn = null;
}

/**
 * Overlay only. Caller may pass excludeBarId (e.g. comments bar id) without
 * this module importing comments.
 */
export function initDocHighlightOverlay({
  getBody,
  getEditArea,
  getIdentityKey,
  excludeBarId = '',
  buttonId,
}: HighlightOverlayConfig) {
  cleanupDocHighlightOverlay();
  const btn = resolveBtn(buttonId);
  _btn = btn;
  let pendingText: string | null = null;
  let pendingOccurrence = 0;
  let pendingBody: Element | null = null;

  const onDocMouseDown = (e: MouseEvent) => {
    if (e.target !== btn) hideBtn();
  };
  document.addEventListener('mousedown', onDocMouseDown);
  _cleanups.push(() => document.removeEventListener('mousedown', onDocMouseDown));

  const onMouseUp = () => {
    const body = getBody?.();
    if (!body) {
      hideBtn();
      return;
    }
    const sel = window.getSelection();
    if (!sel || sel.isCollapsed) {
      hideBtn();
      return;
    }
    const text = sel.toString().trim();
    if (!text) {
      hideBtn();
      return;
    }
    const editArea = getEditArea?.() as HTMLElement | null;
    if (editArea && editArea.style.display !== 'none') return;

    const exclude = excludeBarId ? document.getElementById(excludeBarId) : null;
    if (exclude && exclude.contains(sel.anchorNode)) {
      hideBtn();
      return;
    }
    if (!body.contains(sel.anchorNode)) {
      hideBtn();
      return;
    }

    pendingOccurrence = getOccurrenceIndex(body, sel, text, excludeBarId);
    pendingText = text;
    pendingBody = body;
    const range = sel.getRangeAt(0);
    const rect = range.getBoundingClientRect();
    btn.style.top = `${rect.top - 36}px`;
    btn.style.left = `${rect.left + rect.width / 2 - 30}px`;
    btn.style.display = 'block';
  };
  document.addEventListener('mouseup', onMouseUp);
  _cleanups.push(() => document.removeEventListener('mouseup', onMouseUp));

  const onBtnMouseDown = (e: MouseEvent) => e.preventDefault();
  btn.addEventListener('mousedown', onBtnMouseDown);
  _cleanups.push(() => btn.removeEventListener('mousedown', onBtnMouseDown));

  const onBtnClick = async () => {
    const text = pendingText;
    const occurrence = pendingOccurrence;
    const body = pendingBody || getBody?.();
    hideBtn();
    pendingBody = null;
    window.getSelection()?.removeAllRanges();
    const identityKey = getIdentityKey?.(body) || '';
    if (!text || !identityKey || !body) return;
    try {
      const data = (await api.updateDocHighlights(
        identityKey,
        { text, occurrence },
        nowTs(),
      )) as HighlightsPayload;
      if (!data.ok) throw new Error(data.error || 'failed');
      wrapNthMatch(
        body,
        text,
        occurrence,
        data.id as string,
        (id) => {
          void deleteCachedHighlight({ bodyEl: body, identityKey, excludeBarId, id });
        },
        excludeBarId,
      );
    } catch (e) {
      // @ts-expect-error overlay alert uses e.message
      alert(`Highlight failed: ${e.message}`);
    }
  };
  btn.addEventListener('click', onBtnClick);
  _cleanups.push(() => btn.removeEventListener('click', onBtnClick));
}
