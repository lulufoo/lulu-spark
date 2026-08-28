import type { RouteHandlers } from './hash-router.tsx';

export type RouteHandlerState = {
  handlers: RouteHandlers;
  fallback: string;
};

type Listener = (next: RouteHandlerState) => void;

let current: RouteHandlerState | null = null;
const listeners = new Set<Listener>();

/** boot.ts registers hash → mount functions after App has painted the chrome. */
export function setRouteHandlers(handlers: RouteHandlers, fallback = '#/home') {
  current = { handlers, fallback };
  for (const listen of listeners) listen(current);
}

export function subscribeRouteHandlers(listen: Listener): () => void {
  listeners.add(listen);
  if (current) listen(current);
  return () => {
    listeners.delete(listen);
  };
}
