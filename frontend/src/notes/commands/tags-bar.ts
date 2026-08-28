// @ts-nocheck — host/state snapshots stay unchecked; do not type this file alone.
import { mergeAnnotations, notifyState, state } from '../state/host.ts';
import { TAG_VALUE_MAX_LEN } from '../../host/constants.ts';
import * as api from '../../host/api.ts';
import { applyListFilters, selectDate } from './sidebar.ts';

export function findTagKeyByValue(registry, value) {
  const trimmed = (value || '').trim();
  if (!trimmed) return null;
  const keys = registry?.keys || {};
  let bestKey = null;
  let bestRefs = -1;
  for (const [key, meta] of Object.entries(keys)) {
    if ((meta?.value || '') !== trimmed) continue;
    const refs = meta?.refs ?? 0;
    if (refs > bestRefs) {
      bestRefs = refs;
      bestKey = key;
    }
  }
  return bestKey;
}

function syncEntryTagsFromAnn(entry) {
  const ann = state.index.annotations?.[entry.common_path];
  if (ann?.tags) {
    entry.tags = ann.tags;
    entry.tag_keys = ann.tag_keys;
  } else {
    delete entry.tags;
    delete entry.tag_keys;
  }
}

export async function refreshTagDisplayGlobally() {
  try {
    const [regData, summary] = await Promise.all([
      api.fetchTagsRegistry(),
      api.fetchAnnotationsSummary(),
    ]);
    if (regData?.keys) state.index.tagsRegistry = { keys: regData.keys };
    if (summary && state.index.data) {
      state.index.annotations = summary;
      mergeAnnotations(state.index.data, summary);
      applyListFilters();
      if (state.index.filteredGroups.length > 0 && state.ui.activeDate) {
        const still = state.index.filteredGroups.some((g) => g.date === state.ui.activeDate);
        if (still) selectDate(state.ui.activeDate);
        else selectDate(state.index.filteredGroups[0].date);
      }
    }
    if (state.viewer.entry && !state.viewer.isKb) {
      syncEntryTagsFromAnn(state.viewer.entry);
    }
    notifyState();
  } catch (e) {
    console.error('refreshTagDisplayGlobally failed', e);
  }
}

export async function detachNoteTag(key) {
  const entry = state.viewer.entry;
  if (!entry) return { ok: false, error: 'No entry' };
  const data = await api.tagDetach(entry.common_path, key);
  if (!data.ok) return data;
  await refreshTagDisplayGlobally();
  syncEntryTagsFromAnn(entry);
  notifyState();
  return { ok: true };
}

export async function attachNoteTag(payload) {
  const entry = state.viewer.entry;
  if (!entry) return { ok: false, error: 'No entry' };
  const value = payload.value;
  if (value !== undefined) {
    if (!value) return { ok: false, error: 'Tag text required' };
    if (value.length > TAG_VALUE_MAX_LEN) {
      return { ok: false, error: `Cannot exceed ${TAG_VALUE_MAX_LEN} characters` };
    }
  }
  const data = await api.tagAttach(entry.common_path, payload);
  if (!data.ok) return data;
  await refreshTagDisplayGlobally();
  syncEntryTagsFromAnn(entry);
  notifyState();
  return { ok: true, idempotent: data.idempotent };
}

export async function updateNoteTagValue(key, rawValue) {
  const trimmed = (rawValue || '').trim();
  if (!trimmed) return { ok: false, error: 'Tag text required' };
  if (trimmed.length > TAG_VALUE_MAX_LEN) {
    return { ok: false, error: `Cannot exceed ${TAG_VALUE_MAX_LEN} characters` };
  }
  const data = await api.tagUpdateValue(key, trimmed);
  if (!data.ok) return data;
  await refreshTagDisplayGlobally();
  return { ok: true };
}
