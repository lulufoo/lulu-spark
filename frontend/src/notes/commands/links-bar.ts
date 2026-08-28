// @ts-nocheck — host/state snapshots stay unchecked; do not type this file alone.
import { notifyState, state } from '../state/host.ts';
import * as api from '../../host/api.ts';

function fallbackTitle(url: string) {
  try {
    return decodeURIComponent(new URL(url).pathname.split('/').filter(Boolean).pop()).replace(/\.md$/, '');
  } catch {
    return url;
  }
}

export async function fetchAndCacheLinkTitle(url: string) {
  if (state.index.titleFetchCache.has(url)) return state.index.titleFetchCache.get(url);
  try {
    const data = await api.fetchLinkTitle(url);
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

export async function resolveLinkTitle(url: string) {
  if (state.index.titleFetchCache.has(url)) return state.index.titleFetchCache.get(url);
  try {
    const data = await api.fetchLinkTitle(url);
    return data.title || fallbackTitle(url);
  } catch {
    return fallbackTitle(url);
  }
}

export async function addNoteLink(url: string, title?: string) {
  const entry = state.viewer.entry;
  if (!entry || !url) return { ok: false, error: 'No entry' };
  const existing = entry.links || [];
  if (existing.some((l) => l.url === url)) return { ok: false, error: 'Link already exists' };
  const resolved = title || (await resolveLinkTitle(url));
  const newLinks = [...existing, { url }];
  const data = await api.updateLinks(entry.common_path, newLinks);
  if (!data.ok) return data;
  entry.links = newLinks;
  if (state.viewer.annotation) state.viewer.annotation.links = newLinks;
  state.index.titleFetchCache.set(url, resolved);
  notifyState();
  return { ok: true };
}

export async function removeNoteLink(index: number) {
  const entry = state.viewer.entry;
  if (!entry) return { ok: false, error: 'No entry' };
  const newLinks = (entry.links || []).filter((_, i) => i !== index);
  const data = await api.updateLinks(entry.common_path, newLinks);
  if (!data.ok) return data;
  entry.links = newLinks;
  if (state.viewer.annotation) state.viewer.annotation.links = newLinks;
  notifyState();
  return { ok: true };
}
