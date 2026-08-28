import { useEffect, useSyncExternalStore } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { parseHash } from './router/index.ts';
import { ShellPages } from './shell-pages.tsx';

export type RouteCtx = { name: string; params: Record<string, string> };
export type RouteHandlers = Record<string, (ctx: RouteCtx) => void>;

function subscribeHash(onStoreChange: () => void) {
  window.addEventListener('hashchange', onStoreChange);
  window.addEventListener('popstate', onStoreChange);
  return () => {
    window.removeEventListener('hashchange', onStoreChange);
    window.removeEventListener('popstate', onStoreChange);
  };
}

function getHashSnapshot() {
  return window.location.hash;
}

export function HashRouter({
  handlers,
  fallback = '#/home',
}: {
  handlers: RouteHandlers;
  fallback?: string;
}) {
  const hash = useSyncExternalStore(subscribeHash, getHashSnapshot, getHashSnapshot);
  const route = parseHash(hash) as RouteCtx;
  const pageName = route.name === 'unknown' ? 'home' : route.name;

  useEffect(() => {
    if (route.name === 'unknown') {
      if (window.location.hash !== fallback) {
        window.location.replace(fallback);
      }
      return;
    }
    handlers[route.name]?.(route);
  }, [handlers, fallback, hash]);

  return <ShellPages routeName={pageName} routeParams={route.params} />;
}

let root: Root | null = null;

/** Tests / leftover callers: dispatch lives in App via setRouteHandlers. */
export function mountHashRouter(
  handlers: RouteHandlers,
  options: { fallback?: string } = {},
) {
  let host = document.getElementById('react-root');
  if (!host) {
    host = document.createElement('div');
    host.id = 'react-root';
    host.hidden = true;
    document.body.appendChild(host);
  }
  root?.unmount();
  root = createRoot(host);
  flushSync(() => {
    root!.render(<HashRouter handlers={handlers} fallback={options.fallback} />);
  });
}

export function unmountHashRouter() {
  root?.unmount();
  root = null;
  document.getElementById('react-root')?.remove();
}
