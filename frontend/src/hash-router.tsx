import { useEffect } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { parseHash } from './router/index.ts';

export type RouteCtx = { name: string; params: Record<string, string> };
export type RouteHandlers = Record<string, (ctx: RouteCtx) => void>;

export function HashRouter({
  handlers,
  fallback = '#/home',
}: {
  handlers: RouteHandlers;
  fallback?: string;
}) {
  useEffect(() => {
    const mount = () => {
      const route = parseHash(window.location.hash) as RouteCtx;
      if (route.name === 'unknown') {
        if (window.location.hash !== fallback) {
          window.location.replace(fallback);
        }
        return;
      }
      handlers[route.name]?.(route);
    };
    window.addEventListener('hashchange', mount);
    window.addEventListener('popstate', mount);
    mount();
    return () => {
      window.removeEventListener('hashchange', mount);
      window.removeEventListener('popstate', mount);
    };
  }, [handlers, fallback]);
  return null;
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
