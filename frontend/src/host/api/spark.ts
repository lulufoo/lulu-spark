import { asRecord, type PathArg } from '../api-types.ts';
import { getReadDriver, readGet, writePost } from './transport.ts';

/** Sidebar index of one workspace (the default workspace when `workspaceId` is omitted). */
export async function fetchIndex(workspaceId?: string) {
  const ws = workspaceId ? `workspace_id=${encodeURIComponent(workspaceId)}&` : '';
  return readGet(`/api/docs-index?${ws}_=${Date.now()}`);
}

export async function fetchAnnotationsSummary() {
  try {
    return await readGet('/api/annotations?_=' + Date.now());
  } catch {
    return null;
  }
}

export async function fetchAnnotation(id: PathArg) {
  const res = await getReadDriver().fetchGet(
    `/api/annotation?id=${encodeURIComponent(String(id ?? ''))}`,
  );
  return res.json();
}

export async function fetchConfig() {
  return readGet('/api/config');
}

export async function setConfig(payload?: unknown) {
  return writePost('/api/config', payload || {});
}

export async function fetchFileContent(layer: PathArg, id: PathArg, lang?: PathArg): Promise<string> {
  const langQ = lang ? `&lang=${encodeURIComponent(String(lang))}` : '';
  const data = await readGet(
    `/api/doc-file?layer=${encodeURIComponent(String(layer ?? ''))}&id=${encodeURIComponent(String(id ?? ''))}${langQ}&_=${Date.now()}`
  );
  if (typeof data === 'string') return data;
  const content = asRecord(data)?.content;
  return typeof content === 'string' ? content : '';
}

/** Load notes raster asset via invoke; returns blob: URL (caller may revoke). */
export async function fetchNotesAssetAsBlobUrl(
  layer: string,
  id: string,
  href: string,
  lang?: string,
) {
  const langQ = lang ? `&lang=${encodeURIComponent(lang)}` : '';
  const data = await readGet(
    `/api/doc-asset?layer=${encodeURIComponent(layer)}&id=${encodeURIComponent(id)}&href=${encodeURIComponent(href)}${langQ}&_=${Date.now()}`
  );
  const rec = asRecord(data);
  if (rec?.error) {
    throw new Error(String(rec.error));
  }
  const mime = rec?.mime_type || 'application/octet-stream';
  const b64 = rec?.data_b64;
  if (!b64 || typeof b64 !== 'string') {
    throw new Error('Missing asset payload');
  }
  const binary = atob(b64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  const blob = new Blob([bytes], { type: String(mime) });
  return URL.createObjectURL(blob);
}

export async function fetchLinkTitle(url: string) {
  return readGet(`/api/fetch-title?url=${encodeURIComponent(url)}`);
}

export async function saveFile(layer: PathArg, id: PathArg, content: string, lang?: PathArg) {
  return writePost('/api/save', { layer, id, content, ...(lang ? { lang } : {}) });
}

export async function updateComments(
  id: PathArg,
  layer: PathArg,
  comment: unknown,
  ts?: unknown,
) {
  return writePost('/api/update-comments', {
    id,
    layer,
    comment,
    ts,
  });
}

export async function reorderComments(id: string, layer: string, ids: unknown) {
  return writePost('/api/reorder-comments', {
    id,
    layer,
    ids,
  });
}

export async function updateLinks(id: string, links: unknown) {
  return writePost('/api/update-links', { id, links });
}

export async function fetchTagsRegistry() {
  return readGet('/api/tags/registry');
}

export async function tagAttach(id: string, payload?: Record<string, unknown>) {
  return writePost('/api/tag/attach', {
    id,
    ...payload,
  });
}

export async function tagDetach(id: string, key: string) {
  return writePost('/api/tag/detach', { id, key });
}

export async function tagUpdateValue(key: string, value: unknown) {
  return writePost('/api/tag/update-value', { key, value });
}

export async function setImportance(id: string, importance?: unknown) {
  return writePost('/api/set-importance', {
    id,
    importance: importance || null,
  });
}

export async function setDone(id: string, done: unknown) {
  return writePost('/api/set-done', { id, done });
}

export async function setDocSourceType(id: string, sourceType: string) {
  return writePost('/api/set-doc-source-type', { id, source_type: sourceType });
}

/** Rename the independent document title, without changing its file or body. */
export async function setDocTitle(id: string, title: string) {
  return writePost('/api/set-doc-title', { id, title });
}

export async function deleteEntry(id: PathArg) {
  return writePost('/api/delete', { id });
}

/** What one document did in a sync that did not go as far as it should. */
export type SyncDocumentIssue = {
  id: string;
  op: 'upload' | 'pull';
  error: string;
};

/** The answer of a sync that was made, even when part of it went wrong (see `failed`, `stopped`). */
export type SyncDocumentsResult = {
  ok: true;
  uploaded: number;
  pulled: number;
  skipped: number;
  kept: number;
  gone: number;
  /** Documents the run did not get to because it stopped. */
  not_tried: number;
  failed: SyncDocumentIssue[];
  /** Set when an error that would hit every document ended the run early. */
  stopped: SyncDocumentIssue | null;
  /** True when the user cancelled the run; `not_tried` is what it left for the next one. */
  cancelled: boolean;
};

/**
 * Syncs this machine's documents with the signed-in user's cloud. Throws (with `status`) when no
 * sync was made: 401 not signed in or session not accepted, 409 already running, 502 cloud
 * unavailable, 500 local database.
 */
export async function syncDocuments(): Promise<SyncDocumentsResult> {
  return writePost('/api/cloud-sync', {});
}

/** The answer of a cancel: whether a sync was going on to be cancelled. */
export type CancelSyncResult = { ok: true; cancelling: boolean };

/**
 * Asks the sync that is going on to stop. It answers at once; `syncDocuments` answers, with
 * `cancelled: true`, when the document being transferred is done. `cancelling: false` means no
 * sync was going on, and nothing is remembered for the next one.
 */
export async function cancelSyncDocuments(): Promise<CancelSyncResult> {
  return writePost('/api/cloud-sync-cancel', {});
}

/** What a sync would do now, counted without doing it. */
export type CheckSyncResult = { ok: true; uploads: number; pulls: number };

/**
 * Counts the documents a sync would send and bring back, without syncing. Throws (with `status`)
 * when the count could not be made: 401 not signed in or session not accepted, 502 cloud
 * unavailable, 500 local database. It may be asked while a sync is going on.
 */
export async function checkSyncDocuments(): Promise<CheckSyncResult> {
  return writePost('/api/cloud-sync-check', {});
}

/** One document of a sync run, as the host's record has it. */
export type SyncProgressEntry = {
  id: string;
  /** Title, else file name, else the first characters of the id. */
  name: string;
  op: 'upload' | 'pull';
  state: 'not_started' | 'running' | 'done' | 'failed';
  /** Why it failed, in the words of the log; null unless `state` is `failed`. */
  reason: string | null;
  /** Whether this machine has the document: always for an upload, and for a download once it is here. */
  local: boolean;
};

/** All of the host's record of a sync run, at one moment. */
export type SyncProgressResult = {
  ok: true;
  /** True while a sync is going on; false when `entries` is what the last sync left. */
  running: boolean;
  /** Every upload, then every download, each in order of document id. */
  entries: SyncProgressEntry[];
};

/**
 * Reads the whole record of the sync that is going on, or of the last one, in one call. The host
 * keeps it in memory, clears it when a sync begins and keeps it after the sync ends, so the
 * screen asks again once the sync is over. Replace the list you had with the one you get.
 */
export async function getSyncProgress(): Promise<SyncProgressResult> {
  return writePost('/api/cloud-sync-progress', {});
}

export async function updateHighlight(
  id: string,
  layer: string,
  highlight: unknown,
  ts?: unknown,
) {
  return writePost('/api/update-highlights', {
    id,
    layer,
    highlight,
    ts,
  });
}

export async function fetchDocHighlights(key: string) {
  return readGet(`/api/doc-highlights?key=${encodeURIComponent(key)}`);
}

export async function updateDocHighlights(key: string, highlight: unknown, ts?: unknown) {
  return writePost('/api/doc-highlights', { key, highlight, ts });
}

export async function getDraft(id: string) {
  const res = await getReadDriver().fetchGet(`/api/draft?id=${encodeURIComponent(id)}`);
  if (!res.ok) return { content: '' };
  return res.json();
}

export async function saveDraft(id: PathArg, content: string) {
  return writePost('/api/draft', { id, content });
}

export async function moveToProject(id: string, projectId: string, targetWorkspaceId?: string) {
  const data = await writePost('/api/move-project', {
    id,
    project_id: projectId,
    ...(targetWorkspaceId ? { target_workspace_id: targetWorkspaceId } : {}),
  });
  const rec = asRecord(data);
  if (rec?.error) {
    throw new Error(String(rec.error));
  }
  return data;
}

/** Create a note via HTTP/MCP `create_note` parity.
 * Jot create: `{ body, source_type: 'jot', workspace_id?, project_id? }` (Host synthesize; digest=never);
 * `project_id` is a project id of that workspace; omit it for the default project.
 * Titled jot: `{ title, source_type: 'jot', workspace_id, project_id, content?, folder?, filename? }`.
 * `content` may be empty. `title` and `filename` are stored apart.
 * Path create: `{ source_path, title, digest, asset_paths? }` — no `document` body. */
export async function createNote(payload?: unknown) {
  return writePost('/api/create-note', payload || {});
}

/** Note-create crash buffer under `drafts/docs/<temp_id>`. */
export async function saveDocDraft(tempId: string, content?: string) {
  return writePost('/api/doc-draft', { temp_id: tempId, content: content ?? '' });
}

export async function clearDocDraft(tempId: string) {
  return writePost('/api/doc-draft/clear', { temp_id: tempId });
}

export async function getDocDraft(tempId: string) {
  const res = await getReadDriver().fetchGet(
    `/api/doc-draft?temp_id=${encodeURIComponent(tempId)}`,
  );
  if (!res.ok) return { content: '' };
  return res.json();
}
