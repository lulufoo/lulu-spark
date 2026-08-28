import { getKbHidePattern } from '../../corpus/state/hide-pattern.ts';
import { asRecord, type InvokeResponse, type PathArg } from '../api-types.ts';
import { getReadDriver, normalizeReadError, readGet, writePost } from './transport.ts';

export async function fetchKbFileContent(repo: PathArg, path: PathArg) {
  const res = await getReadDriver().fetchGet(
    `/api/kb/read?repo=${encodeURIComponent(String(repo ?? ''))}&path=${encodeURIComponent(String(path ?? ''))}`
  );
  if (!res.ok) {
    let msg = `HTTP ${res.status}`;
    try {
      const d = asRecord(await res.json());
      if (d?.error) msg = String(d.error);
    } catch {
      /* keep HTTP status message */
    }
    throw new Error(msg);
  }
  return res.json();
}

export async function fetchKbList(repo: string, path = '', mode = 'flat') {
  const params = new URLSearchParams({ repo, path, mode });
  return readGet(`/api/kb/list?${params.toString()}`);
}

const KB_DOC_COUNT_TIMEOUT_MS = 10_000;

export async function fetchKbDocCount(repo: string, hidePattern?: string): Promise<number> {
  const resolvedHide =
    hidePattern !== undefined ? hidePattern : getKbHidePattern();
  const params = new URLSearchParams({ repo });
  if (resolvedHide) {
    params.set('hide_pattern', resolvedHide);
  }
  const path = `/api/kb/doc-count?${params.toString()}`;

  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), KB_DOC_COUNT_TIMEOUT_MS);
  try {
    const res = await Promise.race([
      getReadDriver().fetchGet(path),
      new Promise<never>((_, reject) => {
        controller.signal.addEventListener(
          'abort',
          () => {
            reject(controller.signal.reason ?? new DOMException('Aborted', 'AbortError'));
          },
          { once: true },
        );
      }),
    ]);
    const response = res as InvokeResponse;
    if (!response.ok) {
      let msg = `HTTP ${response.status}`;
      try {
        const d = asRecord(await response.json());
        if (d?.error) msg = String(d.error);
      } catch {
        /* keep HTTP status message */
      }
      throw new Error(msg);
    }
    const data = await response.json();
    const rec = asRecord(data);
    if (rec?.error) {
      throw new Error(typeof rec.error === 'string' ? rec.error : 'Request failed');
    }
    const count = rec?.count;
    return typeof count === 'number' ? count : 0;
  } catch (error) {
    throw normalizeReadError(error);
  } finally {
    clearTimeout(timeoutId);
  }
}

export async function fetchKbAnnotation(repo: PathArg, path: PathArg) {
  const res = await getReadDriver().fetchGet(
    `/api/kb/annotation?repo=${encodeURIComponent(String(repo ?? ''))}&path=${encodeURIComponent(String(path ?? ''))}`
  );
  if (!res.ok) return {};
  return res.json();
}

export async function updateKbComment(repo: PathArg, path: PathArg, comment: unknown, ts?: unknown) {
  return writePost('/api/kb/update-comments', { repo, path, comment, ts });
}

export async function reorderKbComments(repo: PathArg, path: PathArg, ids: unknown) {
  return writePost('/api/kb/reorder-comments', { repo, path, ids });
}

export async function updateKbHighlight(
  repo: PathArg,
  path: PathArg,
  highlight: unknown,
  ts?: unknown,
) {
  return writePost('/api/kb/update-highlights', { repo, path, highlight, ts });
}

export async function updateKbLinks(repo: PathArg, path: PathArg, links: unknown) {
  return writePost('/api/kb/update-links', { repo, path, links });
}

export async function saveKbFile(repo: PathArg, path: PathArg, content: string) {
  return writePost('/api/kb/save', { repo, path, content });
}

export async function commitKbFile(repo: PathArg, message: string) {
  return writePost('/api/kb/commit', { repo, message });
}

export async function fetchKbStatus(repo: PathArg) {
  const res = await getReadDriver().fetchGet(
    `/api/kb/status?repo=${encodeURIComponent(String(repo ?? ''))}`,
  );
  return res.json();
}

export async function fetchKbDiffStatus() {
  return readGet('/api/kb/diff-status?_=' + Date.now());
}

export async function revertKbFile(repo: PathArg, path?: PathArg, type?: PathArg) {
  const body: Record<string, unknown> = { repo };
  if (path) { body.path = path; body.type = type; }
  return writePost('/api/kb/revert', body);
}

export async function reindexKbRepo(repo: string) {
  const { invokeSearch } = await import('../apiClient.ts');
  return invokeSearch('reindexKbRepo', { repo });
}

export async function openItermAt(repo: string) {
  return writePost('/api/open-iterm', { repo });
}

export async function syncKnowledgeCorpus() {
  const { invokeSearch } = await import('../apiClient.ts');
  return invokeSearch('syncKnowledgeCorpus');
}

export async function searchKnowledge(q: string, limit = 10) {
  const res = await getReadDriver().fetchGet(
    `/api/search-knowledge?q=${encodeURIComponent(q)}&limit=${limit}`
  );
  return res.json();
}

export async function reindexKnowledge() {
  const { invokeSearch } = await import('../apiClient.ts');
  return invokeSearch('reindexKnowledge');
}

export async function getReindexStatus() {
  const { invokeSearch } = await import('../apiClient.ts');
  return invokeSearch('getReindexStatus');
}

export async function searchWorkbench(q: string, limit = 10) {
  const res = await getReadDriver().fetchGet(
    `/api/search-workbench?q=${encodeURIComponent(q)}&limit=${limit}`
  );
  return res.json();
}

export async function reindexWorkbench() {
  const { invokeSearch } = await import('../apiClient.ts');
  return invokeSearch('reindexWorkbench');
}

export async function getReindexWorkbenchStatus() {
  const { invokeSearch } = await import('../apiClient.ts');
  return invokeSearch('getReindexWorkbenchStatus');
}

export async function fetchSedimentKbCategories() {
  return readGet('/api/sediment-kb/categories');
}

export async function fetchSedimentKbRepos() {
  return readGet('/api/sediment-kb/repos');
}

export async function addSedimentKbRepo(
  fullName: string,
  categoryId?: string,
  description?: string,
) {
  const body: Record<string, unknown> = { full_name: fullName };
  if (categoryId != null && categoryId !== '') {
    body.category_id = categoryId;
  }
  if (description != null && description !== '') {
    body.description = description;
  }
  return writePost('/api/sediment-kb/repos/add', body);
}

export async function removeSedimentKbRepo(fullName: string) {
  return writePost('/api/sediment-kb/repos/remove', { full_name: fullName });
}

export async function updateSedimentKbRepoCategory(fullName: string, categoryId: string) {
  return writePost('/api/sediment-kb/repos/update-category', {
    full_name: fullName,
    category_id: categoryId,
  });
}

export async function addSedimentKbCategory(name: string) {
  return writePost('/api/sediment-kb/categories/add', { name });
}

export async function renameSedimentKbCategory(id: string, name: string) {
  return writePost('/api/sediment-kb/categories/rename', { id, name });
}

export async function removeSedimentKbCategory(id: string) {
  return writePost('/api/sediment-kb/categories/remove', { id });
}
