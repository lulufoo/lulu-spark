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
  createdAt?: number;
};

export type LiveThinking = {
  text: string;
  ms?: number;
};

export type HubStagedEntry = {
  id: string;
  path: string;
  title: string;
};

export type HubWorkspaceFile = {
  path: string;
  title: string;
};

export type ChannelUnread = {
  docs: boolean;
  read_later: boolean;
};

/** A project of a workspace as the Home sidebar lists it. */
export type HomeProject = {
  id: string;
  title: string;
  /** Id of the configured icon; empty = the default project icon. */
  icon: string;
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
  /** The chats the host lists for the sidebar (the most recent days), newest first. */
  sessions: HubSession[];
  currentSessionId: string;
  messages: HubMessage[];
  staged: HubStagedEntry[];
  workspace: HubWorkspaceFile[];
  hostBound: boolean;
  contextPercent: number | null;
  contextUsage: ContextUsage | null;
  progressByChat: Record<string, string>;
  thinkingByChat: Record<string, LiveThinking>;
  inFlightIds: string[];
  channelUnread: ChannelUnread;
  /** Enabled projects per workspace id, in title order. */
  workspaceProjects: Record<string, HomeProject[]>;
};

function emptyUnread(): ChannelUnread {
  return { docs: false, read_later: false };
}

function emptyState(): HomeState {
  return {
    sessions: [],
    currentSessionId: '',
    messages: [],
    staged: [],
    workspace: [],
    hostBound: false,
    contextPercent: null,
    contextUsage: null,
    progressByChat: Object.create(null) as Record<string, string>,
    thinkingByChat: Object.create(null) as Record<string, LiveThinking>,
    inFlightIds: [],
    channelUnread: emptyUnread(),
    workspaceProjects: {},
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

export function nowUnixSecs() {
  return Math.floor(Date.now() / 1000);
}

export function formatMessageWhen(createdAt: unknown) {
  const ts = Number(createdAt);
  if (!Number.isFinite(ts) || ts <= 0) return '';
  const date = new Date(ts * 1000);
  if (Number.isNaN(date.getTime())) return '';
  const month = date.getMonth() + 1;
  const day = date.getDate();
  const hour = String(date.getHours()).padStart(2, '0');
  const minute = String(date.getMinutes()).padStart(2, '0');
  return `${month}/${day}, ${hour}:${minute}`;
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

/** `today`, `yesterday`, a local date `YYYY-MM-DD`, or `undated` for a chat with no usable time. */
export type SessionGroupKey = string;
export type SessionGroup = { key: SessionGroupKey; label: string; sessions: HubSession[] };

const UNDATED_GROUP: SessionGroup = { key: 'undated', label: 'No date', sessions: [] };

function localDayKey(date: Date) {
  const mm = String(date.getMonth() + 1).padStart(2, '0');
  const dd = String(date.getDate()).padStart(2, '0');
  return `${date.getFullYear()}-${mm}-${dd}`;
}

/** `Oct 13`, or `Oct 13, 2025` when the day is not in the current year. */
function dayHeading(date: Date, now: Date) {
  const withYear = date.getFullYear() !== now.getFullYear();
  return date.toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
    ...(withYear ? { year: 'numeric' } : {}),
  });
}

/**
 * Groups chats under one heading per local calendar day of `updated_at` (Unix seconds): `Today`,
 * `Yesterday`, then the date (`Oct 13`, with the year when it is not the current one). The input
 * order is kept, and so is the order days first appear in; a day with no chats has no heading.
 * A chat with a missing or invalid `updated_at` goes last under `No date`.
 */
export function groupSessionsByDay(
  sessions: HubSession[],
  nowSecs: number = nowUnixSecs(),
): SessionGroup[] {
  const now = new Date(nowSecs * 1000);
  const yesterday = new Date(now);
  // setDate keeps the local calendar day across a daylight-saving change; subtracting 24h would not.
  yesterday.setDate(yesterday.getDate() - 1);
  const todayKey = localDayKey(now);
  const yesterdayKey = localDayKey(yesterday);

  const groups: SessionGroup[] = [];
  const byKey = new Map<string, SessionGroup>();
  const undated: HubSession[] = [];
  for (const session of sessions) {
    const ts = Number(session?.updated_at);
    if (!Number.isFinite(ts) || ts <= 0) {
      undated.push(session);
      continue;
    }
    const day = new Date(ts * 1000);
    const key = localDayKey(day);
    let group = byKey.get(key);
    if (!group) {
      if (key === todayKey) group = { key: 'today', label: 'Today', sessions: [] };
      else if (key === yesterdayKey) group = { key: 'yesterday', label: 'Yesterday', sessions: [] };
      else group = { key, label: dayHeading(day, now), sessions: [] };
      byKey.set(key, group);
      groups.push(group);
    }
    group.sessions.push(session);
  }
  if (undated.length > 0) groups.push({ ...UNDATED_GROUP, sessions: undated });
  return groups;
}

export function hydrateStaged(raw: unknown): HubStagedEntry[] {
  if (!Array.isArray(raw)) return [];
  const out: HubStagedEntry[] = [];
  for (const row of raw) {
    if (!row || typeof row !== 'object') continue;
    const path = String((row as { path?: unknown }).path ?? '').trim();
    if (!path) continue;
    const title = String((row as { title?: unknown }).title ?? '').trim();
    out.push({
      id: String((row as { id?: unknown }).id ?? ''),
      path,
      title: title || path.split('/').pop() || path,
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
      const createdAt = createdAtOf(t);
      return {
        role: row.role,
        text: String(row.content),
        ...(thinking ? { thinking } : {}),
        ...(thinkingMs != null ? { thinkingMs } : {}),
        ...(createdAt != null ? { createdAt } : {}),
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

function createdAtOf(turn: unknown) {
  if (!turn || typeof turn !== 'object') return undefined;
  const raw = (turn as { created_at?: unknown }).created_at;
  if (typeof raw !== 'number' || !Number.isFinite(raw) || raw <= 0) return undefined;
  return Math.trunc(raw);
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
