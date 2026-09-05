import { useSyncExternalStore } from 'react';
import { flushSync } from 'react-dom';
import { createModuleStore } from '../../shared/module-store.ts';

export type HubSession = {
  session_id?: string;
  title?: string;
  updated_at?: number;
};

export type HubMessage = {
  role: string;
  text: string;
  error?: boolean;
};

export type HubStagedEntry = {
  id: string;
  path: string;
  title: string;
  kind?: string;
};

export type ChannelUnread = {
  notes: boolean;
  read_later: boolean;
  todos: boolean;
};

export type HomeState = {
  sessions: HubSession[];
  currentSessionId: string;
  messages: HubMessage[];
  staged: HubStagedEntry[];
  hostBound: boolean;
  progressByChat: Record<string, string>;
  inFlightIds: string[];
  channelUnread: ChannelUnread;
};

function emptyUnread(): ChannelUnread {
  return { notes: false, read_later: false, todos: false };
}

function emptyState(): HomeState {
  return {
    sessions: [],
    currentSessionId: '',
    messages: [],
    staged: [],
    hostBound: false,
    progressByChat: Object.create(null) as Record<string, string>,
    inFlightIds: [],
    channelUnread: emptyUnread(),
  };
}

const homeStore = createModuleStore<HomeState>(emptyState());

export function subscribeHome(onStoreChange: () => void) {
  return homeStore.subscribe(onStoreChange);
}

export function getHomeState() {
  return homeStore.getSnapshot();
}

export function setHomeState(next: HomeState | ((prev: HomeState) => HomeState)) {
  flushSync(() => {
    homeStore.set(next);
  });
}

export function resetHomeState() {
  homeStore.set(emptyState());
}

export function useHomeState() {
  return useSyncExternalStore(homeStore.subscribe, homeStore.getSnapshot, homeStore.getSnapshot);
}

export function composerLocked(state: HomeState) {
  return !state.hostBound || state.inFlightIds.includes(state.currentSessionId);
}

export function composerInputLocked(state: HomeState) {
  return !state.hostBound;
}

export function progressHint(state: HomeState) {
  return state.currentSessionId ? String(state.progressByChat[state.currentSessionId] || '') : '';
}

export function messagePaintKey(state: HomeState) {
  return [
    state.hostBound ? '1' : '0',
    state.currentSessionId,
    ...state.messages.map((m) => `${m.role}\0${m.text}\0${m.error ? '1' : '0'}`),
  ].join('\n');
}

export function formatSessionWhen(updatedAt: unknown) {
  const ts = Number(updatedAt);
  if (!Number.isFinite(ts) || ts <= 0) return '';
  const date = new Date(ts * 1000);
  if (Number.isNaN(date.getTime())) return '';
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  const hour = String(date.getHours()).padStart(2, '0');
  const minute = String(date.getMinutes()).padStart(2, '0');
  return `${year}-${month}-${day} ${hour}:${minute}`;
}

export function sessionListLabel(session: HubSession) {
  const title = String(session?.title || '').trim();
  if (title) return title;
  return formatSessionWhen(session?.updated_at) || 'New conversation';
}

export function hydrateStaged(raw: unknown): HubStagedEntry[] {
  if (!Array.isArray(raw)) return [];
  const out: HubStagedEntry[] = [];
  for (const row of raw) {
    if (!row || typeof row !== 'object') continue;
    const path = String((row as { path?: unknown }).path ?? '').trim();
    if (!path) continue;
    const title = String((row as { title?: unknown }).title ?? '').trim();
    const kind = String((row as { kind?: unknown }).kind ?? '').trim();
    out.push({
      id: String((row as { id?: unknown }).id ?? ''),
      path,
      title: title || path.split('/').pop() || path,
      ...(kind ? { kind } : {}),
    });
  }
  return out;
}

export function hydrateTurns(turns: unknown): HubMessage[] {
  if (!Array.isArray(turns)) return [];
  return turns
    .filter(
      (t) =>
        t &&
        typeof t === 'object' &&
        ((t as HubMessage).role === 'user' || (t as HubMessage).role === 'assistant') &&
        (t as { content?: unknown }).content != null &&
        String((t as { content?: unknown }).content).length > 0,
    )
    .map((t) => ({
      role: (t as HubMessage).role,
      text: String((t as { content: unknown }).content),
    }));
}

export function sessionIdOf(payload: { session_id?: unknown; sessionId?: unknown } | null) {
  if (!payload || typeof payload !== 'object') return '';
  const raw = payload.session_id ?? payload.sessionId;
  return raw == null ? '' : String(raw);
}

/** Next row after `deletedId`; if none, the previous row; if none, empty. */
export function neighborSessionId(sessions: HubSession[], deletedId: string) {
  const ids = sessions.map((s) => String(s.session_id || '')).filter(Boolean);
  const index = ids.indexOf(deletedId);
  if (index < 0) return ids[0] || '';
  return ids[index + 1] || ids[index - 1] || '';
}
