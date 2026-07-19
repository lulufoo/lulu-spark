const DEFAULT_FALLBACK = '#/home';

/** @type {Record<string, (ctx: { name: string, params: Record<string, string> }) => void> | null} */
let handlersRef = null;
let fallbackRef = DEFAULT_FALLBACK;

/**
 * @param {string | undefined} hash
 * @returns {{ name: string, params: Record<string, string> }}
 */
export function parseHash(hash) {
  const raw = hash ?? (typeof window !== 'undefined' ? window.location.hash : '');
  const stripped = raw.startsWith('#') ? raw.slice(1) : raw;
  const path = stripped.replace(/^\/+/, '').replace(/\/+$/, '');

  if (!path) return { name: 'unknown', params: {} };
  if (path === 'home') return { name: 'home', params: {} };
  if (path === 'read-later') return { name: 'read-later', params: {} };
  if (path === 'workbench' || path.startsWith('workbench?')) {
    const queryString = path.includes('?') ? path.slice(path.indexOf('?') + 1) : '';
    const params = {};
    if (queryString) {
      const searchParams = new URLSearchParams(queryString);
      if (searchParams.has('date')) {
        params.date = searchParams.get('date') ?? '';
      }
      if (searchParams.has('note')) {
        params.note = searchParams.get('note') ?? '';
      }
      if (searchParams.has('layer')) {
        params.layer = searchParams.get('layer') ?? '';
      }
    }
    return { name: 'workbench', params };
  }

  if (path === 'plan-tasks' || path.startsWith('plan-tasks?')) {
    const queryString = path.includes('?') ? path.slice(path.indexOf('?') + 1) : '';
    const params = {};
    if (queryString) {
      const searchParams = new URLSearchParams(queryString);
      if (searchParams.has('master')) {
        params.master = searchParams.get('master') ?? '';
      }
      if (searchParams.has('sub')) {
        params.sub = searchParams.get('sub') ?? '';
      }
    }
    return { name: 'plan-tasks', params };
  }

  if (path === 'corpus' || path === 'corpus/pick') return { name: 'corpus-doc', params: { repo: '' } };

  if (path.startsWith('corpus/')) {
    const repoPart = path.slice('corpus/'.length);
    if (repoPart) {
      try {
        const [repoEncoded, ...queryParts] = repoPart.split('?');
        const queryString = queryParts.length > 0 ? queryParts.join('?') : '';
        const params = { repo: decodeURIComponent(repoEncoded) };
        if (queryString) {
          const searchParams = new URLSearchParams(queryString);
          if (searchParams.has('path')) {
            params.path = searchParams.get('path') ?? '';
          }
        }
        return { name: 'corpus-doc', params };
      } catch {
        return { name: 'unknown', params: {} };
      }
    }
  }

  return { name: 'unknown', params: {} };
}

/** @param {string} hash */
export function normalizeHash(hash) {
  if (parseHash(hash).name === 'unknown') return DEFAULT_FALLBACK;
  return hash.startsWith('#') ? hash : `#/${hash.replace(/^\/+/, '')}`;
}

function mountCurrentRoute() {
  const route = parseHash();
  if (route.name === 'unknown') {
    if (window.location.hash !== fallbackRef) {
      window.location.replace(fallbackRef);
    }
    return;
  }
  handlersRef?.[route.name]?.(route);
}

/**
 * @param {Record<string, (ctx: { name: string, params: Record<string, string> }) => void>} handlers
 * @param {{ fallback?: string }} [options]
 */
export function initRouter(handlers, options = {}) {
  handlersRef = handlers;
  fallbackRef = options.fallback ?? DEFAULT_FALLBACK;
  window.addEventListener('hashchange', mountCurrentRoute);
  window.addEventListener('popstate', mountCurrentRoute);
  mountCurrentRoute();
}

/** @param {string} hash */
export function navigate(hash) {
  window.location.hash = hash;
}

/** Tracks list→note pushes so exit can prefer history.back. */
let noteNavDepth = 0;

/**
 * @param {{ date?: string, note?: string, layer?: string }} [params]
 * @returns {string}
 */
export function buildWorkbenchHash({ date, note, layer } = {}) {
  const searchParams = new URLSearchParams();
  if (date) searchParams.set('date', date);
  if (note) searchParams.set('note', note);
  if (layer) searchParams.set('layer', layer);
  const qs = searchParams.toString();
  return qs ? `#/workbench?${qs}` : '#/workbench';
}

/**
 * List→note: write date+note(+optional layer) via navigate (browser history).
 * Failure: leave location unchanged (stay list / safe empty).
 * @param {{ date?: string, note?: string, layer?: string }} [params]
 * @returns {boolean}
 */
export function navigateToNote({ date, note, layer } = {}) {
  if (!date || !note) return false;
  try {
    navigate(buildWorkbenchHash({ date, note, layer }));
    noteNavDepth += 1;
    return true;
  } catch {
    return false;
  }
}

/**
 * Exit note / create: prefer history.back when a list→note push is available;
 * otherwise strip note and keep date. May mutate hash (exit-must-not-change-hash abolished).
 * Create-in-progress: call onClearCreate, land on list without writing a temp note.
 * @param {{ date?: string, onClearCreate?: () => void }} [options]
 */
export function navigateBackToList({ date, onClearCreate } = {}) {
  if (typeof onClearCreate === 'function') {
    onClearCreate();
  }

  const route = parseHash();
  const resolvedDate = date || route.params?.date || '';
  const hasNote = Boolean(route.params?.note);

  const canHistoryBack =
    hasNote &&
    noteNavDepth > 0 &&
    typeof window !== 'undefined' &&
    typeof window.history?.back === 'function' &&
    (window.history.length ?? 0) > 1;

  if (canHistoryBack) {
    noteNavDepth -= 1;
    window.history.back();
    return;
  }

  noteNavDepth = 0;
  navigate(buildWorkbenchHash({ date: resolvedDate }));
}
