import { knowledgeDocKey, notesDocKey } from '../../doc-editor/identity.ts';
import { state } from '../../host/state.ts';

const NOTES_MARKERS = ['/notes/raw/', '/notes/digest/'] as const;

export function notesCommonPathFromAbs(path: string): string {
  const normalized = path.replace(/\\/g, '/');
  for (const marker of NOTES_MARKERS) {
    const idx = normalized.lastIndexOf(marker);
    if (idx >= 0) return normalized.slice(idx + marker.length);
  }
  return '';
}

function knowledgeRelFromAbs(path: string): string {
  const normalized = path.replace(/\\/g, '/');
  const root = String(state.ui.knowledgeRoot || '')
    .replace(/\\/g, '/')
    .replace(/\/+$/, '');
  if (!root) return '';
  if (normalized === root) return '';
  if (normalized.startsWith(`${root}/`)) return normalized.slice(root.length + 1);
  return '';
}

/** Build FilePopup identity from Stage kind + path. Bare stage (no kind) → no key. */
export function stagedIdentityKey(kind: string | undefined, path: string): string | undefined {
  const k = String(kind || '').trim();
  const abs = String(path || '').trim();
  if (!k || !abs) return undefined;
  if (k === 'notes') {
    const common = notesCommonPathFromAbs(abs);
    return common ? notesDocKey(common) : undefined;
  }
  if (k === 'knowledge') {
    const rel = knowledgeRelFromAbs(abs);
    if (!rel) return undefined;
    const segs = rel.split('/').filter(Boolean);
    if (segs.length === 0) return undefined;
    if (segs.length === 1) return `knowledge:${segs[0]}`;
    return knowledgeDocKey(segs[0], segs.slice(1).join('/'));
  }
  return undefined;
}
