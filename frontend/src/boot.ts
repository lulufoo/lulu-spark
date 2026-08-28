// @ts-nocheck — large port; DOM wiring stays unchecked like checkJs:false.
import { state, loadDiffStatus, mergeAnnotations, notifyState } from './host/state.ts'
import { escHtml } from './shared/utils.ts'
import { LAYERS, setGithubUserUrl } from './host/constants.ts'
import * as api from './host/api.ts'
import { buildGroups, renderSidebar, selectDate, applyListFilters, selectTag } from './notes/sidebar.tsx'
import { initSidebarResize } from './notes/sidebar-resize.ts'
import { openCreateNote } from './notes/viewer.ts'
import './notes/comments.tsx'
import './corpus/corpus-viewer.ts'
import './app-shell/commit-dialog.tsx'
import './app-shell/qr-dialog.tsx'
import { navigate, navigateToNote } from './router/index.ts'
import { setRouteHandlers } from './route-handlers.ts'
import { openReadLaterDialog } from './read-later/dialog.tsx'
import { getBaselineEntries } from './home-entry-shell/entry-config.ts'
import { createContentRegistry } from './home-entry-shell/content-registry.ts'
import { mountHomeEntryShell } from './home-entry-shell/shell.tsx'
import { createReadLaterContentAdapter } from './read-later/assistant.tsx'
import { createTodoTaskContentAdapter } from './todo-task/assistant.tsx'
import { createNotesContentAdapter } from './notes/assistant.tsx'
import { createBuildersContentAdapter } from './builders/assistant.tsx'
import { setWorkbenchBinding } from './todo-task/binding.ts'
import { initHeaderSync } from './app-shell/header-sync.ts'
import { normalizeCorpusIndex } from './corpus/corpus-index.ts'
import './app-shell/sediment-kb.tsx'
import { initTooltip } from './app-shell/tooltip.ts'
import './app-shell/skills-dialog.tsx'
import {
  getHomeEntryShell,
  mountCorpusDocRoute,
  mountHomeRoute,
  mountReadLaterRoute,
  mountTodoTasksRoute,
  mountWorkbench,
  setHomeEntryShell,
  wrapRouteMount,
} from './app-shell/routes.ts'

const titleCache = state.index.titleCache;

// ── Fetch index.json ───────────────────────────────────────────────────────

async function loadIndex({ managedBtn = false } = {}) {
  try {
    const data = await api.fetchIndex();
    state.index.data = normalizeCorpusIndex(data);
    state.index.groupedByDate = buildGroups(state.index.data);
    state.ui.activeTopic = null;
    state.ui.activeTagKey = null;
    applyListFilters();
    renderSidebar();
    await Promise.all([loadDiffStatus(), loadAnnotationsSummary(), loadTagsRegistry()]);
    applyListFilters();
    renderSidebar();
    const savedDate = sessionStorage.getItem('cta_active_date');
    const targetDate = (savedDate && state.index.filteredGroups.find(g => g.date === savedDate))
      ? savedDate
      : (state.index.filteredGroups.length > 0 ? state.index.filteredGroups[0].date : null);
    if (targetDate) selectDate(targetDate);
    notifyState();
  } catch (e) {
    showError(`Could not load index.json: ${e.message}`);
  }
}

async function loadAnnotationsSummary() {
  try {
    const summary = await api.fetchAnnotationsSummary();
    if (!summary) return;
    state.index.annotations = summary;
    if (state.index.data) mergeAnnotations(state.index.data, summary);
  } catch {
    // Non-fatal: annotations are optional
  }
}

async function loadTagsRegistry() {
  try {
    const data = await api.fetchTagsRegistry();
    if (data?.keys) state.index.tagsRegistry = { keys: data.keys };
  } catch (e) {
    console.error('loadTagsRegistry failed', e);
  }
}

// ── Error display ──────────────────────────────────────────────────────────

function showError(msg) {
  const status = document.getElementById('status');
  status.style.display = '';
  const errDiv = document.createElement('div');
  errDiv.className = 'error-msg';
  errDiv.innerHTML = escHtml(msg) + '<br>';
  const retryBtn = document.createElement('button');
  retryBtn.textContent = 'Retry';
  retryBtn.addEventListener('click', loadIndex);
  errDiv.appendChild(retryBtn);
  status.innerHTML = '';
  status.appendChild(errDiv);
  document.getElementById('date-heading').style.display = 'none';
  document.getElementById('doc-list').innerHTML = '';
}

// ── Pull project ───────────────────────────────────────────────────────────

async function pullProject() {
  const btn = document.getElementById('btn-pull');
  btn.disabled = true;
  btn.textContent = 'Updating…';
  try {
    const data = await api.pullProject();
    if (data.error) throw new Error((data.error || '') + (data.stderr ? '\n' + data.stderr : ''));

    titleCache.clear();
    if (state.index.data) {
      for (const entry of Object.values(state.index.data)) {
        LAYERS.forEach(l => delete entry[`_unreachable_${l}`]);
      }
    }
    await loadIndex();
  } catch (e) {
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

api.fetchConfig().then(d => {
  state.ui.workbenchKnowledgeRoot = d.workbench_knowledge_root || '';
  state.ui.knowledgeCorpusRoot = d.knowledge_corpus_root || '';
  state.ui.githubUserUrl = d.github_user_url || '';
  setGithubUserUrl(d.github_user_url);
}).catch(() => {});
api.fetchTopics().then(data => {
  const descMap = {};
  const repoMap = {};
  for (const t of (data.topics || [])) {
    if (!t.repo) continue;
    const key = t.dir || t.repo.split('/')[1];
    if (t.description) descMap[key] = t.description;
    repoMap[key] = `https://github.com/${t.repo}`;
  }
  state.index.topicDescriptions = descMap;
  state.index.topicRepos = repoMap;
}).catch(() => {});
initSidebarResize();
initTooltip();
loadIndex();

setRouteHandlers({
  workbench: wrapRouteMount('workbench', (route) => mountWorkbench(route)),
  home: wrapRouteMount('home', mountHomeRoute),
  'corpus-doc': wrapRouteMount('corpus-doc', mountCorpusDocRoute),
  'read-later': wrapRouteMount('read-later', mountReadLaterRoute),
  'todo-tasks': wrapRouteMount('todo-tasks', mountTodoTasksRoute),
}, '#/home');

function closeNoteAssistantPanel() {
  getHomeEntryShell()?.forceRecoverA('leave-host');
}

function openCreateNoteFromFab(opts = {}) {
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
void setWorkbenchBinding();

/** Present-before-listen race buffer (L11-AR). Cleared on pull / successful open. */
let pendingPresentOpen = false;

/**
 * ai-assistant:opened consumer (main window).
 * surface===Present → Home chat; ensure/session payloads sync only (no overlay).
 * @param {unknown} payload
 */
function handleAiAssistantOpenedPayload(payload) {
  if (!payload || typeof payload !== 'object') return;
  if (/** @type {{ surface?: unknown }} */ (payload).surface === 'Present') {
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
  const onOpened = (event) => {
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
    renderSidebar();
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

document.addEventListener('cta:filter-tag', ({ detail }) => {
  if (detail?.key) selectTag(detail.key);
});

// ── Global search navigation ───────────────────────────────────────────────
document.addEventListener('cta:open-entry', ({ detail }) => {
  closeNoteAssistantPanel();
  if (!detail?.common_path) return
  const allEntries = Object.values(state.index.data || {})
  let entry = allEntries.find(e => e.common_path === detail.common_path)
  // Fallback: detail.common_path may be a zh translation file (e.g. from a
  // stale Meilisearch index). Resolve it to the main entry via translations.zh.
  if (!entry) entry = allEntries.find(e => e.translations?.zh === detail.common_path)
  if (!entry) return
  const date = entry.created_at
    ? entry.created_at.slice(0, 8)
    : (state.ui?.activeDate || '')
  if (!date) return
  selectDate(date)
  // Optional layer only when entry synthesizes one; consumer defaults when absent.
  const params = { date, note: entry.common_path }
  if (detail.layer) params.layer = detail.layer
  navigateToNote(params)
});

document.addEventListener('cta:open-kb-doc', ({ detail }) => {
  if (!detail || !detail.repo || !detail.path) return
  navigate('#/corpus/' + encodeURIComponent(detail.repo) + '?path=' + encodeURIComponent(detail.path))
});
