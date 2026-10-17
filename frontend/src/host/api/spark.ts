import { asRecord, type PathArg } from '../api-types.ts';
import { getReadDriver, readGet, writePost } from './transport.ts';

/** Sidebar index of one workspace (the default workspace when `workspace` is omitted). */
export async function fetchIndex(workspace?: string) {
  const ws = workspace ? `workspace=${encodeURIComponent(workspace)}&` : '';
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

export async function deleteEntry(id: PathArg) {
  return writePost('/api/delete', { id });
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

export async function moveToProject(id: string, newProject: string) {
  const data = await writePost('/api/move-project', { id, new_project: newProject });
  const rec = asRecord(data);
  if (rec?.error) {
    throw new Error(String(rec.error));
  }
  return data;
}

/** Create a note via HTTP/MCP `create_note` parity.
 * Jot create: `{ body, source_type: 'jot', workspace?, project? }` (Host synthesize; digest=never);
 * `project` is a project folder of that workspace, `inbox` when left out.
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
