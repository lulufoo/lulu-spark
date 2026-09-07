import { notifyState, state } from '../state/host.ts';
import * as api from '../../host/api.ts';

type KbLink = { url: string };
type KbAnnotation = { links?: KbLink[] };

function viewer() {
  return state.viewer as unknown as { kbRepo: string; kbPath: string; annotation: KbAnnotation };
}

function fallbackTitle(url: string) {
  try {
    return decodeURIComponent(new URL(url).pathname.split('/').filter(Boolean).pop() || '').replace(/\.md$/, '');
  } catch {
    return url;
  }
}

export async function resolveKbLinkTitle(url: string) {
  if (state.index.titleFetchCache.has(url)) return state.index.titleFetchCache.get(url) as string;
  try {
    const data = (await api.fetchLinkTitle(url)) as { title?: string };
    return data.title || fallbackTitle(url);
  } catch {
    return fallbackTitle(url);
  }
}

export async function fetchAndCacheKbLinkTitle(url: string) {
  if (state.index.titleFetchCache.has(url)) return state.index.titleFetchCache.get(url) as string;
  const title = await resolveKbLinkTitle(url);
  state.index.titleFetchCache.set(url, title);
  notifyState();
  return title;
}

export async function addKbLink(url: string, title?: string) {
  const { kbRepo, kbPath, annotation } = viewer();
  const existing = annotation.links || [];
  if (existing.some((l) => l.url === url)) return { ok: false, error: 'Link already exists' };
  const resolved = title || (await resolveKbLinkTitle(url));
  const newLinks = [...existing, { url }];
  const data = (await api.updateKbLinks(kbRepo, kbPath, newLinks)) as { ok?: boolean; error?: string };
  if (!data.ok) return data;
  annotation.links = newLinks;
  state.index.titleFetchCache.set(url, resolved);
  notifyState();
  return { ok: true };
}

export async function removeKbLink(index: number) {
  const { kbRepo, kbPath, annotation } = viewer();
  const newLinks = (annotation.links || []).filter((_, i) => i !== index);
  const data = (await api.updateKbLinks(kbRepo, kbPath, newLinks)) as { ok?: boolean; error?: string };
  if (!data.ok) return data;
  annotation.links = newLinks;
  notifyState();
  return { ok: true };
}
