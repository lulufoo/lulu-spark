import { act } from 'react';
import { releaseBoardSketches } from '../../frontend/src/board/commands/sketch-links.ts';

export const PAGE = 'https://luluboard.app/view#b:b_0123abcd';
export const ALIAS = 'https://luluboard.app/view?alias=1#b:b_0123abcd';
export const OTHER = 'https://luluboard.app/view#b:b_89abcdef';

export function identity(url = PAGE, options = {}) {
  return {
    credential_scope: 'scope-a', generation: 1,
    diagram_id: new URL(url).hash.slice(3).split(/[&/?\s]/)[0], diagram_generation: 1,
    ...options,
  };
}

export function source(options = {}) {
  const { url = PAGE, html = '<p>Cached</p>', status = 'sketch', ...rest } = options;
  return {
    ...identity(url), status, version: 1, title: 'Cached title',
    ...(status === 'sketch' ? { content_b64: btoa(html) } : {}), ...rest,
  };
}

/** Metadata supplied by the Host since LW805, used by the older UI fixtures. */
export function withBoardIdentity(reply, url) {
  if (['sketch', 'not_sketch', 'invalid_source'].includes(reply?.status)) {
    return { ...identity(url), version: 1, ...reply };
  }
  return reply;
}

export function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}

export function page(url = PAGE, text = 'Original Board link') {
  const host = document.createElement('div');
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.textContent = text;
  host.appendChild(anchor);
  document.body.appendChild(host);
  return host;
}

export function readers(options = {}) {
  const callbacks = new Set();
  let subscriptions = 0;
  let stops = 0;
  const listenInvalidated = async (callback) => {
    subscriptions += 1;
    callbacks.add(callback);
    return () => { stops += 1; callbacks.delete(callback); };
  };
  return {
    readCached: async () => ({ status: 'cache_miss' }),
    readRemote: async () => source(),
    listenInvalidated,
    emit(event) { for (const callback of callbacks) callback(event); },
    get subscriptions() { return subscriptions; },
    get stops() { return stops; },
    ...options,
  };
}

export async function flush() {
  await act(async () => {});
}

export async function ready(frame) {
  const posted = [];
  frame.contentWindow.postMessage = (data) => posted.push(data);
  await act(async () => {
    window.dispatchEvent(new MessageEvent('message', {
      data: { type: 'lulu-sandbox:ready' }, source: frame.contentWindow,
    }));
  });
  return posted;
}

export function cleanPages() {
  for (const child of [...document.body.children]) releaseBoardSketches(child);
  document.body.innerHTML = '';
  document.documentElement.dataset.theme = 'light';
}
