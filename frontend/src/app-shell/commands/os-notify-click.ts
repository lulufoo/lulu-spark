import { openWorkbenchScheme } from '../../router/scheme.ts';

type ClickPayload = { scheme?: string };

let unlisten: (() => void) | null = null;

function getTauriListen() {
  if (typeof window === 'undefined') return null;
  const listen = window.__TAURI__?.event?.listen;
  return typeof listen === 'function' ? listen : null;
}

export function handleOsNotifyClicked(payload?: ClickPayload): boolean {
  if (!payload || typeof payload !== 'object') return false;
  const value = payload.scheme;
  if (typeof value !== 'string' || !value) return false;
  return openWorkbenchScheme(value);
}

export function startOsNotifyClickHub(): void {
  if (typeof unlisten === 'function') {
    unlisten();
    unlisten = null;
  }

  const listen = getTauriListen();
  if (!listen) return;

  void listen('os-notification:clicked', (event) => {
    handleOsNotifyClicked(event?.payload as ClickPayload | undefined);
  }).then((fn) => {
    unlisten = fn;
  });
}
