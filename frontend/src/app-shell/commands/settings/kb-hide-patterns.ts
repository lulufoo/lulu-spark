import * as api from '../../../host/api.ts';
import {
  emitHidePatternChanged,
  setKbHidePatternsCache,
  validateKbHidePattern,
  type KbHidePatternRow,
} from '../../../knowledge/state/hide-pattern.ts';
import { kbHidePatternsStore, patchKbHidePatterns } from '../../state/settings/kb-hide-patterns.ts';

function asRows(data: unknown): KbHidePatternRow[] {
  const rec = data as { patterns?: Array<Partial<KbHidePatternRow>>; error?: string };
  if (rec?.error) throw new Error(rec.error);
  const rows: KbHidePatternRow[] = [];
  for (const row of rec.patterns || []) {
    const id = typeof row.id === 'string' ? row.id.trim() : '';
    const pattern = typeof row.pattern === 'string' ? row.pattern : '';
    if (!id) continue;
    rows.push({ id, pattern });
  }
  return rows;
}

function applyRows(rows: KbHidePatternRow[]) {
  setKbHidePatternsCache(rows);
  patchKbHidePatterns({ rows, error: '', busy: false });
  emitHidePatternChanged();
}

export async function loadKbHidePatterns() {
  patchKbHidePatterns({ error: '', busy: true });
  try {
    applyRows(asRows(await api.fetchKbHidePatterns()));
  } catch (e) {
    patchKbHidePatterns({
      error: (e as Error).message || String(e),
      busy: false,
    });
  }
}

export function startKbHidePatternEdit(id: string) {
  const row = kbHidePatternsStore.getSnapshot().rows.find((r) => r.id === id);
  if (!row) return;
  patchKbHidePatterns({ editingId: id, draft: row.pattern, error: '' });
}

export function setKbHidePatternDraft(draft: string) {
  patchKbHidePatterns({ draft });
}

export function setKbHidePatternAddValue(addValue: string) {
  patchKbHidePatterns({ addValue });
}

export function cancelKbHidePatternEdit() {
  patchKbHidePatterns({ editingId: '', draft: '' });
}

export async function saveKbHidePatternEdit() {
  const snap = kbHidePatternsStore.getSnapshot();
  if (!snap.editingId) return;
  const validation = validateKbHidePattern(snap.draft);
  if (!validation.ok) {
    patchKbHidePatterns({ error: `Invalid regex: ${validation.error}` });
    return;
  }
  patchKbHidePatterns({ busy: true, error: '' });
  try {
    applyRows(asRows(await api.updateKbHidePattern(snap.editingId, snap.draft.trim())));
    patchKbHidePatterns({ editingId: '', draft: '' });
  } catch (e) {
    patchKbHidePatterns({
      error: (e as Error).message || String(e),
      busy: false,
    });
  }
}

export async function addKbHidePatternRow() {
  const value = kbHidePatternsStore.getSnapshot().addValue;
  const validation = validateKbHidePattern(value);
  if (!value.trim()) return;
  if (!validation.ok) {
    patchKbHidePatterns({ error: `Invalid regex: ${validation.error}` });
    return;
  }
  patchKbHidePatterns({ busy: true, error: '' });
  try {
    applyRows(asRows(await api.addKbHidePattern(value.trim())));
    patchKbHidePatterns({ addValue: '' });
  } catch (e) {
    patchKbHidePatterns({
      error: (e as Error).message || String(e),
      busy: false,
    });
  }
}

export async function removeKbHidePatternRow(id: string) {
  patchKbHidePatterns({ busy: true, error: '' });
  try {
    applyRows(asRows(await api.removeKbHidePattern(id)));
    const snap = kbHidePatternsStore.getSnapshot();
    if (snap.editingId === id) {
      patchKbHidePatterns({ editingId: '', draft: '' });
    }
  } catch (e) {
    patchKbHidePatterns({
      error: (e as Error).message || String(e),
      busy: false,
    });
  }
}
