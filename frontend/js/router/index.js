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
  if (path === 'workbench') return { name: 'workbench', params: {} };
  if (path === 'corpus' || path === 'corpus/pick') return { name: 'corpus-doc', params: { repo: '' } };

  if (path.startsWith('corpus/')) {
    const repoPart = path.slice('corpus/'.length);
    if (repoPart) {
      try {
        return { name: 'corpus-doc', params: { repo: decodeURIComponent(repoPart) } };
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
