import { useSyncExternalStore } from 'react';
import { flushSync } from 'react-dom';
import { createModuleStore } from '../../shared/module-store.ts';

export type HubSession = {
  session_id?: string;
  title?: string;
  updated_at?: number;
  llm?: string;
};

export type HubMessage = {
  role: string;
  text: string;
  error?: boolean;
  thinking?: string;
  thinkingMs?: number;
};

export type LiveThinking = {
  text: string;
  ms?: number;
};

export type HubStagedSource = {
  kind: string;
  id: string;
};

export type HubStagedEntry = {
  id: string;
  path: string;
  title: string;
  source?: HubStagedSource;
};

export type ChannelUnread = {
  notes: boolean;
  read_later: boolean;
};

export type ContextUsageCategory = {
  id: string;
  tokens: number;
};

export type ContextUsage = {
  totalTokens: number;
  windowTokens: number;
  categories: ContextUsageCategory[];
};

export type HomeState = {
  sessions: HubSession[];
  currentSessionId: string;
  messages: HubMessage[];
  staged: HubStagedEntry[];
  hostBound: boolean;
  contextPercent: number | null;
  contextUsage: ContextUsage | null;
  progressByChat: Record<string, string>;
  thinkingByChat: Record<string, LiveThinking>;
  inFlightIds: string[];
  channelUnread: ChannelUnread;
};

function emptyUnread(): ChannelUnread {
  return { notes: false, read_later: false };
}

function emptyState(): HomeState {
  return {
    sessions: [],
    currentSessionId: '',
    messages: [],
    staged: [],
    hostBound: false,
    contextPercent: null,
    contextUsage: null,
    progressByChat: Object.create(null) as Record<string, string>,
    thinkingByChat: Object.create(null) as Record<string, LiveThinking>,
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
    ...state.messages.map(
      (m) => `${m.role}\0${m.text}\0${m.error ? '1' : '0'}\0${m.thinking || ''}\0${m.thinkingMs ?? ''}`,
    ),
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
    const rawSource = (row as { source?: { kind?: unknown; id?: unknown } }).source;
    const sourceKind = String(rawSource?.kind ?? '').trim();
    const sourceId = String(rawSource?.id ?? '').trim();
    const source =
      sourceKind && sourceId ? { kind: sourceKind, id: sourceId } : undefined;
    out.push({
      id: String((row as { id?: unknown }).id ?? ''),
      path,
      title: title || path.split('/').pop() || path,
      ...(source ? { source } : {}),
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
    .map((t) => {
      const row = t as { role: string; content: unknown; thinking?: unknown; thinking_ms?: unknown };
      const thinking = String(row.thinking ?? '').trim();
      const thinkingMs =
        typeof row.thinking_ms === 'number' && Number.isFinite(row.thinking_ms)
          ? row.thinking_ms
          : undefined;
      return {
        role: row.role,
        text: String(row.content),
        ...(thinking ? { thinking } : {}),
        ...(thinkingMs != null ? { thinkingMs } : {}),
      };
    });
}

export function liveThinkingOf(state: HomeState) {
  const id = state.currentSessionId;
  if (!id) return null;
  const live = state.thinkingByChat[id];
  return live && live.text ? live : null;
}

export function thinkingTitle(ms?: number, live?: boolean) {
  const underOne = ms == null || ms < 1000;
  if (live) return underOne ? 'Thinking' : `Thinking · ${Math.round(ms / 1000)}s`;
  return underOne ? 'Thought briefly' : `Thought for ${Math.round(ms / 1000)}s`;
}

export function contextUsageFrom(payload: object | null): ContextUsage | null {
  if (!payload || !('context_usage' in payload)) return null;
  const raw = (payload as { context_usage?: unknown }).context_usage;
  if (!raw || typeof raw !== 'object') return null;
  const body = raw as {
    total_tokens?: unknown;
    window_tokens?: unknown;
    categories?: unknown;
  };
  if (typeof body.total_tokens !== 'number' || typeof body.window_tokens !== 'number') return null;
  const categories = Array.isArray(body.categories)
    ? body.categories.flatMap((item) => {
        if (!item || typeof item !== 'object') return [];
        const row = item as { id?: unknown; tokens?: unknown };
        if (typeof row.id !== 'string' || typeof row.tokens !== 'number' || row.tokens <= 0) return [];
        return [{ id: row.id, tokens: row.tokens }];
      })
    : [];
  return {
    totalTokens: body.total_tokens,
    windowTokens: body.window_tokens,
    categories,
  };
}

export function contextPercentFrom(payload: object | null) {
  if (!payload || !('context_percent' in payload)) return null;
  const raw = (payload as { context_percent?: unknown }).context_percent;
  if (typeof raw !== 'number' || !Number.isFinite(raw)) return null;
  return Math.round(raw);
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
