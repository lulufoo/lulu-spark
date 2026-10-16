import { state, mergeAnnotations, notifyState } from './host/state.ts';
import { LAYERS } from './host/constants.ts';
import * as api from './host/api.ts';
import type { HostIndexAnnotation, HostNoteEntry } from './host/snapshot-types.ts';
import { buildGroups, selectDate, applyListFilters, selectTag } from './notes/commands/sidebar.ts';
import './notes/ui/comments.tsx';
import './knowledge/viewer.ts';
import { navigate, navigateToNote } from './router/index.ts';
import { setRouteHandlers } from './route-handlers.ts';
import { setSparkBinding } from './app-shell/commands/spark-binding.ts';
import { normalizeKnowledgeIndex } from './knowledge/state/index.ts';
import './app-shell/ui/settings/sediment-kb.tsx';
import { initTooltip } from './app-shell/ui/tooltip.ts';
import {
  getHomeEntryShell,
  mountKnowledgeDocRoute,
  mountHomeRoute,
  mountReadLaterRoute,
  mountSpark,
  wrapRouteMount,
} from './app-shell/routes.ts';
import type { SettingsConfig } from './app-shell/state/types.ts';
import { applyTheme, parseThemeId, watchSystemTheme } from './theme.ts';
import { startSparkSchemeOpenedHub, type SparkEnvelope } from './router/scheme.ts';
import { startOsNotifyClickHub } from './app-shell/commands/os-notify-click.ts';
import { handleNotesOsNotifyEnvelope } from './notes/commands/os-notify.ts';
import { handleReadLaterOsNotifyEnvelope } from './read-later/commands/os-notify.ts';
import { logNotifyHop, parseTraceId, TRACE_PARAM } from './router/notify-trace.ts';

type NotesCategoriesPayload = {
  categories?: Array<{ id?: string; folder?: string; title?: string; description?: string }>;
};

type TagsRegistryPayload = {
  keys?: Record<string, { value?: string; refs?: number }>;
};

// ── Fetch the notes index ──────────────────────────────────────────────────────

async function loadIndex({ managedBtn = false }: { managedBtn?: boolean } = {}) {
  void managedBtn;
  try {
    const data = await api.fetchIndex();
    state.index.data = normalizeKnowledgeIndex(data) as Record<string, HostNoteEntry>;
    state.index.groupedByDate = buildGroups(state.index.data);
    state.ui.activeTopic = null;
    state.ui.activeTagKey = null;
    state.ui.loadError = null;
    applyListFilters();
    await Promise.all([loadAnnotationsSummary(), loadTagsRegistry()]);
    applyListFilters();
    const savedDate = sessionStorage.getItem('cta_active_date');
    const targetDate = (savedDate && state.index.filteredGroups.find((g) => g.date === savedDate))
      ? savedDate
      : (state.index.filteredGroups.length > 0 ? state.index.filteredGroups[0].date : null);
    if (targetDate) selectDate(targetDate);
    notifyState();
  } catch (e) {
    // @ts-expect-error boot copy scan uses e.message
    showError(`Could not load notes: ${e.message}`);
  }
}

async function loadAnnotationsSummary() {
  try {
    const summary = await api.fetchAnnotationsSummary();
    if (!summary) return;
    state.index.annotations = summary as Record<string, HostIndexAnnotation>;
    if (state.index.data) mergeAnnotations(state.index.data, summary as Record<string, HostIndexAnnotation>);
  } catch {
    // Non-fatal: annotations are optional
  }
}

async function loadTagsRegistry() {
  try {
    const data = (await api.fetchTagsRegistry()) as TagsRegistryPayload;
    if (data?.keys) state.index.tagsRegistry = { keys: data.keys };
  } catch (e) {
    console.error('loadTagsRegistry failed', e);
  }
}

// ── Error display ──────────────────────────────────────────────────────────

function showError(msg: string) {
  state.ui.loadError = msg;
  notifyState();
}

// ── cta:reload ─────────────────────────────────────────────────────────────

document.addEventListener('cta:reload', () => loadIndex());

// ── Init ───────────────────────────────────────────────────────────────────

watchSystemTheme();
api.fetchConfig().then((d) => {
  const cfg = d as SettingsConfig;
  state.ui.sparkRoot = cfg.spark_root || '';
  state.ui.knowledgeRoot = cfg.knowledge_root || '';
  const theme = parseThemeId(cfg.theme);
  if (theme) applyTheme(theme);
}).catch(() => {});
api.fetchNotesCategories().then((data) => {
  const payload = data as NotesCategoriesPayload;
  const descMap: Record<string, string> = {};
  const titleMap: Record<string, string> = {};
  for (const c of payload.categories || []) {
    const id = typeof c.id === 'string' ? c.id.trim() : '';
    const folder =
      typeof c.folder === 'string' && c.folder.trim() ? c.folder.trim() : id;
    if (!folder) continue;
    titleMap[folder] = typeof c.title === 'string' ? c.title : folder;
    descMap[folder] = typeof c.description === 'string' ? c.description : '';
  }
  state.index.topicTitles = titleMap;
  state.index.topicDescriptions = descMap;
  state.index.topicRepos = {};
}).catch(() => {});
initTooltip();
loadIndex();

setRouteHandlers({
  spark: wrapRouteMount('spark', (route) => mountSpark(route)),
  home: wrapRouteMount('home', mountHomeRoute),
  'knowledge-doc': wrapRouteMount('knowledge-doc', mountKnowledgeDocRoute),
  'read-later': wrapRouteMount('read-later', mountReadLaterRoute),
}, '#/home');

function closeNoteAssistantPanel() {
  // @ts-expect-error leave-host source scan requires forceRecoverA(
  getHomeEntryShell()?.forceRecoverA('leave-host');
}

console.info('[DEBUG-assistant] boot: setSparkBinding');
void setSparkBinding().then((result) => {
  console.info('[DEBUG-assistant] boot: setSparkBinding result', result);
});

/** Present-before-listen race buffer (L11-AR). Cleared on pull / successful open. */
let pendingPresentOpen = false;

/**
 * ai-assistant:opened consumer (main window).
 * surface===Present → Home chat; ensure/session payloads sync only (no overlay).
 */
function handleAiAssistantOpenedPayload(payload: unknown) {
  if (!payload || typeof payload !== 'object') return;
  if ((payload as { surface?: unknown }).surface === 'Present') {
    pendingPresentOpen = false;
    navigate('#/home');
  }
}

/** Pull frontend pending + Host pending_present once after mount/listen. */
async function pullPendingPresentOpen() {
  if (pendingPresentOpen) {
    pendingPresentOpen = false;
    navigate('#/home');
  }
}

function registerAiAssistantOpenedListener() {
  const onOpened = (event: { payload?: unknown }) => {
    handleAiAssistantOpenedPayload(event?.payload);
  };
  const tryAttach = () => {
    const listen = typeof window !== 'undefined' && window.__TAURI__?.event?.listen;
    if (typeof listen !== 'function') return false;
    void listen('ai-assistant:opened', onOpened).then(() => {
      void pullPendingPresentOpen();
    });
    return true;
  };
  if (tryAttach()) {
    void pullPendingPresentOpen();
    return;
  }
  let attempts = 0;
  const timer = setInterval(() => {
    if (tryAttach() || ++attempts >= 40) {
      clearInterval(timer);
      void pullPendingPresentOpen();
    }
  }, 50);
}

registerAiAssistantOpenedListener();

export function startOsNotifyHub() {
  const tryAttach = () => {
    const listen = typeof window !== 'undefined' && window.__TAURI__?.event?.listen;
    if (typeof listen !== 'function') return false;
    void listen('message-center:changed', (event: { payload?: unknown }) => {
      const payload = event?.payload as SparkEnvelope;
      const params = payload?.params;
      const trace = parseTraceId(
        params && typeof params === 'object' ? params[TRACE_PARAM] : null,
      );
      logNotifyHop('ui.received', trace, {
        business: payload?.business,
        action: payload?.action,
      });
      void handleNotesOsNotifyEnvelope(payload);
      void handleReadLaterOsNotifyEnvelope(payload);
    });
    return true;
  };
  if (tryAttach()) return;
  let attempts = 0;
  const timer = setInterval(() => {
    if (tryAttach() || ++attempts >= 40) clearInterval(timer);
  }, 50);
}

startOsNotifyHub();

function startOsNotifyClickHubWhenReady() {
  const tryStart = () => {
    const listen = typeof window !== 'undefined' && window.__TAURI__?.event?.listen;
    if (typeof listen !== 'function') return false;
    startOsNotifyClickHub();
    return true;
  };
  if (tryStart()) return;
  let attempts = 0;
  const timer = setInterval(() => {
    if (tryStart() || ++attempts >= 40) clearInterval(timer);
  }, 50);
}

startOsNotifyClickHubWhenReady();

function startSparkSchemeOpenedHubWhenReady() {
  const tryStart = () => {
    const listen = typeof window !== 'undefined' && window.__TAURI__?.event?.listen;
    if (typeof listen !== 'function') return false;
    startSparkSchemeOpenedHub();
    return true;
  };
  if (tryStart()) return;
  let attempts = 0;
  const timer = setInterval(() => {
    if (tryStart() || ++attempts >= 40) clearInterval(timer);
  }, 50);
}

startSparkSchemeOpenedHubWhenReady();

document.addEventListener('cta:filter-tag', (event) => {
  const detail = (event as CustomEvent<{ key?: string }>).detail;
  if (detail?.key) selectTag(detail.key);
});

// ── Global search navigation ───────────────────────────────────────────────
document.addEventListener('cta:open-entry', (event) => {
  const detail = (event as CustomEvent<{ id?: string; layer?: string }>).detail;
  closeNoteAssistantPanel();
  const id = detail?.id;
  if (!id) return;
  const data = state.index.data || {};
  const entry = data[id] ?? Object.values(data).find((e) => e._id === id || e.id === id);
  if (!entry) return;
  const date = entry.created_at
    ? entry.created_at.slice(0, 8)
    : (state.ui?.activeDate || '');
  if (!date) return;
  selectDate(date);
  const params: { date: string; note: string; layer?: string } = { date, note: id };
  params.layer = detail.layer || (LAYERS.find((l) => entry.layers?.includes(l)) ?? 'raw');
  navigateToNote(params);
});

document.addEventListener('cta:open-kb-doc', (event) => {
  const detail = (event as CustomEvent<{ repo?: string; path?: string }>).detail;
  if (!detail || !detail.repo || !detail.path) return;
  navigate('#/knowledge/' + encodeURIComponent(detail.repo) + '?path=' + encodeURIComponent(detail.path));
});
