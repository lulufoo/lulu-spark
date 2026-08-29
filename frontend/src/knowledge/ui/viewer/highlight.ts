import { state } from '../../state/host.ts'
import { renderKbComments } from '../comments.tsx'
import { renderKbLinksBar } from '../links-bar.tsx'
import { renderMermaidBlocks } from '../../../shared/mermaid-render.ts'
import { knowledgeDocKey } from '../../../doc-editor/identity.ts'
import {
  applyCachedHighlights,
  initDocHighlightOverlay,
  cleanupDocHighlightOverlay,
} from '../../../doc-editor/highlights.ts'
import { renderDocMarkdown } from '../../../doc-editor/view.tsx'

const KB_BODY_ID = 'kb-md-body'
const KB_HIGHLIGHT_BTN_ID = 'kb-highlight-add-btn'

let _kbHighlightRoot: HTMLElement | null = null

export function kbIdentityKey() {
  const { kbRepo, kbPath } = state.viewer
  if (!kbRepo || !kbPath) return ''
  return knowledgeDocKey(kbRepo, kbPath)
}

export function kbBodyEl(root?: Element | null) {
  if (root) return root.querySelector('.kb-reader-body') || root.querySelector(`#${KB_BODY_ID}`)
  return document.getElementById(KB_BODY_ID)
}

export function applyKbHighlights() {
  const body = kbBodyEl(_kbHighlightRoot)
  const identityKey = kbIdentityKey()
  if (!body || !identityKey) return
  void applyCachedHighlights({
    bodyEl: body,
    identityKey,
    excludeBarId: 'kb-md-comments-bar',
  })
}

export function cleanupKbHighlightUI() {
  cleanupDocHighlightOverlay()
  _kbHighlightRoot = null
}

export function initKbHighlightUI(container?: Element | null) {
  let root = container
  if (!root) {
    const legacyBody = document.getElementById(KB_BODY_ID)
    root = legacyBody?.closest('.kb-reader') ?? legacyBody?.parentElement
    if (!root) return
  }
  _kbHighlightRoot = root as HTMLElement
  initDocHighlightOverlay({
    getBody: () => kbBodyEl(_kbHighlightRoot),
    getEditArea: () =>
      (root.querySelector('.kb-reader-edit-area') as HTMLElement | null) ||
      document.getElementById('kb-md-edit-area'),
    getIdentityKey: () => kbIdentityKey(),
    excludeBarId: 'kb-md-comments-bar',
    buttonId: KB_HIGHLIGHT_BTN_ID,
  })
  applyKbHighlights()
}

// ── postProcessLinks (KB) ──────────────────────────────────────────────────
export function postProcessKbLinks(container: Element) {
  container.querySelectorAll('a[href]').forEach((a) => {
    const el = a as HTMLAnchorElement
    const href = el.getAttribute('href');
    if (!href || href.startsWith('#')) return;
    if (href.startsWith('http://') || href.startsWith('https://')) {
      el.target = '_blank';
      el.rel = 'noopener noreferrer';
    }
  });
}

export async function renderKbMdBody(text: string, bodyEl?: HTMLElement | null) {
  const body = bodyEl ?? document.getElementById('kb-md-body');
  if (!body) return;
  renderDocMarkdown(body, text);
  postProcessKbLinks(body);
  await renderMermaidBlocks(body);
  renderKbComments(state.viewer.annotation as { comments?: { id: string; text?: string; ts?: string }[] });
  void applyKbHighlights();
  renderKbLinksBar();
}
