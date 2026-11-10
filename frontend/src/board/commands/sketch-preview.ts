export const SANDBOX_SRC = 'https://sandbox.luluboard.app/';
export const SANDBOX_ALLOW = 'allow-scripts';

const GRAPH_ID = /^b_[0-9a-f]{8}$/;
const MAX_EDGE = 32768;

export type SketchAppearance = 'light' | 'dark';

export type SandboxEvent =
  | { kind: 'ready' }
  | { kind: 'error' }
  | { kind: 'size'; width: number; height: number };

export function renderMessage(html: string, appearance: SketchAppearance) {
  return {
    type: 'lulu-sandbox:render',
    html,
    appearance: appearance === 'dark' ? 'dark' : 'light',
  };
}

export function appearanceMessage(appearance: SketchAppearance) {
  return {
    type: 'lulu-sandbox:appearance',
    appearance: appearance === 'dark' ? 'dark' : 'light',
  };
}

function validEdge(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value) && value > 0 && value <= MAX_EDGE;
}

export function readSandboxMessage(data: unknown): SandboxEvent | null {
  if (!data || typeof data !== 'object') return null;
  const rec = data as { type?: unknown; width?: unknown; height?: unknown };
  if (rec.type === 'lulu-sandbox:ready') return { kind: 'ready' };
  if (rec.type === 'lulu-sandbox:error') return { kind: 'error' };
  if (rec.type === 'lulu-sandbox:size' && validEdge(rec.width) && validEdge(rec.height)) {
    return { kind: 'size', width: Math.ceil(rec.width), height: Math.ceil(rec.height) };
  }
  return null;
}

export function isBoardPage(raw: string): boolean {
  let url: URL;
  try {
    url = new URL(raw);
  } catch {
    return false;
  }
  if (url.protocol !== 'https:' || url.hostname !== 'luluboard.app') return false;
  if (url.username || url.password || (url.port !== '' && url.port !== '443')) return false;
  const hash = url.hash.startsWith('#') ? url.hash.slice(1) : url.hash;
  if (!hash.startsWith('b:')) return false;
  const rest = hash.slice(2);
  const token = rest.split(/[&/?\s]/)[0] ?? '';
  return GRAPH_ID.test(token);
}

export async function openBoardPage(url: string) {
  if (!isBoardPage(url)) return;
  const opener = window.__TAURI__?.opener;
  const openUrl = opener?.openUrl;
  if (typeof openUrl === 'function') {
    await openUrl.call(opener, url);
    return;
  }
  window.open(url, '_blank', 'noopener,noreferrer');
}
