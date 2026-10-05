import { notifyState, state } from '../state/host.ts';
import * as api from '../../host/api.ts';

function fallbackTitle(url: string) {
  try {
    const last = new URL(url).pathname.split('/').filter(Boolean).pop() || '';
    return decodeURIComponent(last).replace(/\.md$/, '');
  } catch {
    return url;
  }
}

type OkResult = { ok: boolean; error?: string };

export async function fetchAndCacheLinkTitle(url: string) {
  if (state.index.titleFetchCache.has(url)) return state.index.titleFetchCache.get(url) ?? fallbackTitle(url);
  try {
    const data = (await api.fetchLinkTitle(url)) as { title?: string };
    const title = data.title || fallbackTitle(url);
    state.index.titleFetchCache.set(url, title);
    notifyState();
    return title;
  } catch {
    const title = fallbackTitle(url);
    state.index.titleFetchCache.set(url, title);
    notifyState();
    return title;
  }
}

export async function removeNoteLink(index: number) {
  const entry = state.viewer.entry;
  if (!entry) return { ok: false, error: 'No entry' };
  const newLinks = (entry.links || []).filter((_, i) => i !== index);
  const data = (await api.updateLinks(entry.common_path, newLinks)) as OkResult;
  if (!data.ok) return data;
  entry.links = newLinks;
  if (state.viewer.annotation) state.viewer.annotation.links = newLinks;
  notifyState();
  return { ok: true };
}
