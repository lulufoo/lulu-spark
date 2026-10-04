import { refreshNotesIndex } from '../../notes/commands/reload-index.ts';
import { extractTraceFromScheme, logNotifyHop } from '../../router/notify-trace.ts';
import { openSparkScheme, parseSparkScheme } from '../../router/scheme.ts';

type ClickPayload = { scheme?: string };

let unlisten: (() => void) | null = null;

function getTauriListen() {
  if (typeof window === 'undefined') return null;
  const listen = window.__TAURI__?.event?.listen;
  return typeof listen === 'function' ? listen : null;
}

export async function handleOsNotifyClicked(payload?: ClickPayload): Promise<boolean> {
  if (!payload || typeof payload !== 'object') return false;
  const value = payload.scheme;
  if (typeof value !== 'string' || !value) return false;
  const scheme = value;
  logNotifyHop('click.to_route', extractTraceFromScheme(scheme), {
    outcome: 'ok',
    scheme,
  });
  if (openSparkScheme(scheme)) return true;
  if (parseSparkScheme(scheme)?.kind !== 'notes-open') return false;
  await refreshNotesIndex();
  return openSparkScheme(scheme);
}

export function startOsNotifyClickHub(): void {
  if (typeof unlisten === 'function') {
    unlisten();
    unlisten = null;
  }

  const listen = getTauriListen();
  if (!listen) return;

  void listen('os-notification:clicked', (event) => {
    void handleOsNotifyClicked(event?.payload as ClickPayload | undefined);
  }).then((fn) => {
    unlisten = fn;
  });
}
