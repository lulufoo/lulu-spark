import * as api from '../../../host/api.ts';
import { state } from '../../../host/state.ts';
import {
  emptyHidePatternsSnap,
  hidePatternsStore,
  patchHidePatterns,
} from '../../state/settings/hide-patterns.ts';

function asRows(data: unknown): api.HidePatternRow[] {
  const rec = data as { patterns?: Array<Partial<api.HidePatternRow>>; error?: string };
  if (rec?.error) throw new Error(rec.error);
  const rows: api.HidePatternRow[] = [];
  for (const row of rec.patterns || []) {
    const id = typeof row.id === 'string' ? row.id.trim() : '';
    const pattern = typeof row.pattern === 'string' ? row.pattern : '';
    if (id) rows.push({ id, pattern });
  }
  return rows;
}

/** Early, friendly check; the host stays the authority on what compiles. */
export function validateHidePattern(pattern: string): { ok: true } | { ok: false; error: string } {
  const value = pattern.trim();
  if (!value) return { ok: true };
  try {
    new RegExp(value);
    return { ok: true };
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : 'Invalid regular expression' };
  }
}

function message(e: unknown) {
  return (e as Error).message || String(e);
}

/** Rules hide documents at the source, so the page of that workspace is rebuilt after a change. */
function announceRulesChanged(workspaceId: string) {
  if (workspaceId === state.workspaces.current) {
    document.dispatchEvent(new CustomEvent('cta:reload'));
  }
}

/** The rule list is per workspace: switching the settings nav item starts from a clean tab. */
export async function selectHidePatternsWorkspace(workspaceId: string) {
  hidePatternsStore.set({ ...emptyHidePatternsSnap(), workspaceId, busy: true });
  await loadHidePatterns(workspaceId);
}

/** A late answer for a workspace that is no longer selected is dropped. */
export async function loadHidePatterns(workspaceId: string) {
  patchHidePatterns({ error: '', busy: true });
  try {
    const rows = asRows(await api.fetchHidePatterns(workspaceId));
    if (hidePatternsStore.getSnapshot().workspaceId !== workspaceId) return;
    patchHidePatterns({ rows, error: '', busy: false });
  } catch (e) {
    if (hidePatternsStore.getSnapshot().workspaceId !== workspaceId) return;
    patchHidePatterns({ error: message(e), busy: false });
  }
}

/** Run one write for `workspaceId`; on success replace the rows, on failure keep them and show why. */
async function mutate(workspaceId: string, call: () => Promise<unknown>, after?: () => void) {
  patchHidePatterns({ busy: true, error: '' });
  try {
    const rows = asRows(await call());
    if (hidePatternsStore.getSnapshot().workspaceId !== workspaceId) return;
    patchHidePatterns({ rows, error: '', busy: false });
    after?.();
    announceRulesChanged(workspaceId);
  } catch (e) {
    if (hidePatternsStore.getSnapshot().workspaceId !== workspaceId) return;
    patchHidePatterns({ error: message(e), busy: false });
  }
}

export function startHidePatternEdit(id: string) {
  const row = hidePatternsStore.getSnapshot().rows.find((r) => r.id === id);
  if (!row) return;
  patchHidePatterns({ editingId: id, draft: row.pattern, error: '' });
}

export function setHidePatternDraft(draft: string) {
  patchHidePatterns({ draft });
}

export function setHidePatternAddValue(addValue: string) {
  patchHidePatterns({ addValue });
}

export function cancelHidePatternEdit() {
  patchHidePatterns({ editingId: '', draft: '' });
}

export async function saveHidePatternEdit() {
  const snap = hidePatternsStore.getSnapshot();
  if (!snap.editingId || snap.busy) return;
  const validation = validateHidePattern(snap.draft);
  if (!validation.ok) {
    patchHidePatterns({ error: `Invalid regex: ${validation.error}` });
    return;
  }
  await mutate(
    snap.workspaceId,
    () => api.updateHidePattern(snap.workspaceId, snap.editingId, snap.draft.trim()),
    () => patchHidePatterns({ editingId: '', draft: '' }),
  );
}

export async function addHidePatternRow() {
  const snap = hidePatternsStore.getSnapshot();
  if (!snap.addValue.trim() || snap.busy) return;
  const validation = validateHidePattern(snap.addValue);
  if (!validation.ok) {
    patchHidePatterns({ error: `Invalid regex: ${validation.error}` });
    return;
  }
  await mutate(
    snap.workspaceId,
    () => api.addHidePattern(snap.workspaceId, snap.addValue.trim()),
    () => patchHidePatterns({ addValue: '' }),
  );
}

export async function removeHidePatternRow(id: string) {
  const snap = hidePatternsStore.getSnapshot();
  await mutate(
    snap.workspaceId,
    () => api.removeHidePattern(snap.workspaceId, id),
    () => {
      if (hidePatternsStore.getSnapshot().editingId === id) {
        patchHidePatterns({ editingId: '', draft: '' });
      }
    },
  );
}
