import { state, loadDiffStatus, mergeAnnotations, notifyState } from './host/state.ts';
import { LAYERS, setGithubUserUrl } from './host/constants.ts';
import * as api from './host/api.ts';
import type { HostIndexAnnotation, HostNoteEntry } from './host/snapshot-types.ts';
import { buildGroups, selectDate, applyListFilters, selectTag } from './notes/commands/sidebar.ts';
import { openCreateNote } from './notes/viewer.ts';
import './notes/ui/comments.tsx';
import './knowledge/viewer.ts';
import './app-shell/ui/workbench-commit-dialog.tsx';
import './app-shell/ui/qr-dialog.tsx';
import { navigate, navigateToNote } from './router/index.ts';
import { setRouteHandlers } from './route-handlers.ts';
import { openReadLaterDialog } from './read-later/commands/dialog.ts';
import { getBaselineEntries } from './home-entry-shell/entry-config.ts';
import { createContentRegistry } from './home-entry-shell/content-registry.ts';
import { mountHomeEntryShell } from './home-entry-shell/shell.tsx';
import { createReadLaterContentAdapter } from './read-later/ui/assistant.tsx';
import { createTodoTaskContentAdapter } from './todo-task/ui/assistant.tsx';
import { createNotesContentAdapter } from './notes/ui/assistant.tsx';
import { createBuildersContentAdapter } from './builders/ui/assistant.tsx';
import { setWorkbenchBinding } from './todo-task/commands/binding.ts';
import { initHeaderSync } from './app-shell/commands/header-sync.ts';
import { normalizeKnowledgeIndex } from './knowledge/state/index.ts';
import './app-shell/ui/settings/sediment-kb.tsx';
import { initTooltip } from './app-shell/ui/tooltip.ts';
import './app-shell/ui/skills-dialog.tsx';
import {
  getHomeEntryShell,
  mountKnowledgeDocRoute,
  mountHomeRoute,
  mountReadLaterRoute,
  mountTodoTasksRoute,
  mountWorkbench,
  setHomeEntryShell,
  wrapRouteMount,
} from './app-shell/routes.ts';
import type { SettingsConfig } from './app-shell/state/types.ts';

const titleCache = state.index.titleCache;

type NotesCategoriesPayload = {
  categories?: Array<{ id?: string; folder?: string; title?: string; description?: string }>;
};

type TagsRegistryPayload = {
  keys?: Record<string, { value?: string; refs?: number }>;
};

type PullPayload = {
  error?: string;
  stderr?: string;
};

// ── Fetch index.json ───────────────────────────────────────────────────────

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
    await Promise.all([loadDiffStatus(), loadAnnotationsSummary(), loadTagsRegistry()]);
    applyListFilters();
    const savedDate = sessionStorage.getItem('cta_active_date');
    const targetDate = (savedDate && state.index.filteredGroups.find((g) => g.date === savedDate))
      ? savedDate
      : (state.index.filteredGroups.length > 0 ? state.index.filteredGroups[0].date : null);
    if (targetDate) selectDate(targetDate);
    notifyState();
  } catch (e) {
    // @ts-expect-error boot copy scan uses e.message
    showError(`Could not load index.json: ${e.message}`);
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

// ── Pull project ───────────────────────────────────────────────────────────

async function pullProject() {
  const btn = document.getElementById('btn-pull') as HTMLButtonElement;
  btn.disabled = true;
  btn.textContent = 'Updating…';
  try {
    const data = (await api.pullProject()) as PullPayload;
    if (data.error) throw new Error((data.error || '') + (data.stderr ? '\n' + data.stderr : ''));

    titleCache.clear();
    if (state.index.data) {
      for (const entry of Object.values(state.index.data)) {
        LAYERS.forEach((l) => delete (entry as Record<string, unknown>)[`_unreachable_${l}`]);
      }
    }
    await loadIndex();
  } catch (e) {
    // @ts-expect-error boot copy scan uses e.message
    alert(`Update failed: ${e.message}`);
  } finally {
    btn.disabled = false;
    btn.textContent = '↓ Update project';
  }
}

initHeaderSync({ pullProject, loadIndex });

// ── cta:reload ─────────────────────────────────────────────────────────────

document.addEventListener('cta:reload', () => loadIndex());

// ── Init ───────────────────────────────────────────────────────────────────

api.fetchConfig().then((d) => {
  const cfg = d as SettingsConfig;
  state.ui.workbenchRoot = cfg.workbench_root || '';
  state.ui.knowledgeRoot = cfg.knowledge_root || '';
  state.ui.githubUserUrl = cfg.github_user_url || '';
  setGithubUserUrl(cfg.github_user_url);
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
  workbench: wrapRouteMount('workbench', (route) => mountWorkbench(route)),
  home: wrapRouteMount('home', mountHomeRoute),
  'knowledge-doc': wrapRouteMount('knowledge-doc', mountKnowledgeDocRoute),
  'read-later': wrapRouteMount('read-later', mountReadLaterRoute),
  'todo-tasks': wrapRouteMount('todo-tasks', mountTodoTasksRoute),
}, '#/home');

function closeNoteAssistantPanel() {
  // @ts-expect-error leave-host source scan requires forceRecoverA(
  getHomeEntryShell()?.forceRecoverA('leave-host');
}

function openCreateNoteFromFab(opts: { temp_id?: string } = {}) {
  closeNoteAssistantPanel();
  const temp_id =
    opts?.temp_id ||
    (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function'
      ? crypto.randomUUID()
      : `note-${Date.now()}`);
  // Create shares micro-route authority with open: location omits note.
  const date = state.ui?.activeDate || '';
  navigate(date ? `#/workbench?date=${encodeURIComponent(date)}` : '#/workbench');
  return openCreateNote({ temp_id });
}

// SK-P3: single shell mount + content adapters in the shared content slot.
const homeEntryRegistry = createContentRegistry();
homeEntryRegistry.register('read-later', createReadLaterContentAdapter());
homeEntryRegistry.register('todo-task', createTodoTaskContentAdapter());
homeEntryRegistry.register('notes', createNotesContentAdapter());
homeEntryRegistry.register('builders', createBuildersContentAdapter());

const homeEntryShell = mountHomeEntryShell(document.body, {
  config: getBaselineEntries(),
  registry: homeEntryRegistry,
  host: {
    navigate,
    openReadLater: openReadLaterDialog,
    openCreateNote: openCreateNoteFromFab,
  },
});
setHomeEntryShell(homeEntryShell);
console.info('[DEBUG-assistant] boot: setWorkbenchBinding');
void setWorkbenchBinding().then((result) => {
  console.info('[DEBUG-assistant] boot: setWorkbenchBinding result', result);
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

function registerTagsReconciledListener() {
  const onReconciled = async () => {
    await Promise.all([loadTagsRegistry(), loadAnnotationsSummary()]);
    applyListFilters();
    notifyState();
  };
  const tryAttach = () => {
    const listen = typeof window !== 'undefined' && window.__TAURI__?.event?.listen;
    if (typeof listen !== 'function') return false;
    void listen('tags:reconciled', onReconciled);
    return true;
  };
  if (tryAttach()) return;
  let attempts = 0;
  const timer = setInterval(() => {
    if (tryAttach() || ++attempts >= 40) clearInterval(timer);
  }, 50);
}

registerTagsReconciledListener();

document.addEventListener('cta:filter-tag', (event) => {
  const detail = (event as CustomEvent<{ key?: string }>).detail;
  if (detail?.key) selectTag(detail.key);
});

// ── Global search navigation ───────────────────────────────────────────────
document.addEventListener('cta:open-entry', (event) => {
  const detail = (event as CustomEvent<{ common_path?: string; layer?: string }>).detail;
  closeNoteAssistantPanel();
  if (!detail?.common_path) return;
  const allEntries = Object.values(state.index.data || {});
  let entry = allEntries.find((e) => e.common_path === detail.common_path);
  // Fallback: detail.common_path may be a zh translation file (e.g. from a
  // stale index). Resolve it to the main entry via translations.zh.
  if (!entry) entry = allEntries.find((e) => e.translations?.zh === detail.common_path);
  if (!entry) return;
  const date = entry.created_at
    ? entry.created_at.slice(0, 8)
    : (state.ui?.activeDate || '');
  if (!date) return;
  selectDate(date);
  // Optional layer only when entry synthesizes one; consumer defaults when absent.
  const params: { date: string; note: string; layer?: string } = { date, note: entry.common_path };
  if (detail.layer) params.layer = detail.layer;
  navigateToNote(params);
});

document.addEventListener('cta:open-kb-doc', (event) => {
  const detail = (event as CustomEvent<{ repo?: string; path?: string }>).detail;
  if (!detail || !detail.repo || !detail.path) return;
  navigate('#/knowledge/' + encodeURIComponent(detail.repo) + '?path=' + encodeURIComponent(detail.path));
});
