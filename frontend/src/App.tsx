import { useEffect, useState } from 'react';
import { HashRouter, type RouteHandlers } from './hash-router.tsx';
import { subscribeRouteHandlers } from './route-handlers.ts';
import { Shell } from './shell.tsx';

export function App() {
  const [handlers, setHandlers] = useState<RouteHandlers | null>(null);
  const [fallback, setFallback] = useState('#/home');

  useEffect(() => subscribeRouteHandlers((next) => {
    setHandlers(next.handlers);
    setFallback(next.fallback);
  }), []);

  return (
    <div id="root-shell">
      <Shell />
      {handlers ? <HashRouter handlers={handlers} fallback={fallback} /> : null}
    </div>
  );
}
