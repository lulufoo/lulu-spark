import { createElement } from 'react';
import { flushSync } from 'react-dom';
import { createRoot, type Root } from 'react-dom/client';
import { fetchBoardSketch } from '../../host/api/board-sketch.ts';
import { SketchPreview } from '../ui/sketch-preview.tsx';
import { isBoardPage, type SketchAppearance } from './sketch-preview.ts';

type Reader = (url: string) => Promise<unknown>;

type Slot = {
  unmount: () => void;
  paint: (appearance: SketchAppearance) => void;
};

type Session = {
  gen: number;
  slots: Slot[];
  container: Element;
  read: Reader;
};

const sessions = new WeakMap<Element, Session>();
const live = new Set<Session>();
let watchingTheme = false;

function currentAppearance(): SketchAppearance {
  return document.documentElement.dataset.theme === 'dark' ? 'dark' : 'light';
}

function ensure(container: Element): Session {
  let session = sessions.get(container);
  if (!session) {
    session = { gen: 0, slots: [], container, read: fetchBoardSketch };
    sessions.set(container, session);
    live.add(session);
  }
  return session;
}

function clearSlots(session: Session) {
  for (const slot of session.slots) slot.unmount();
  session.slots = [];
}

export function releaseBoardSketches(container: Element | null | undefined) {
  if (!container) return;
  const session = sessions.get(container);
  if (!session) return;
  session.gen += 1;
  clearSlots(session);
}

export function invalidateBoardSketches() {
  for (const session of [...live]) {
    if (!session.container.isConnected) {
      releaseBoardSketches(session.container);
      live.delete(session);
      continue;
    }
    void mountBoardSketches(session.container, session.read);
  }
}

function watchTheme() {
  if (watchingTheme || typeof MutationObserver !== 'function') return;
  watchingTheme = true;
  const observer = new MutationObserver(() => {
    const appearance = currentAppearance();
    for (const session of live) {
      for (const slot of session.slots) slot.paint(appearance);
    }
  });
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme'],
  });
}

function sketchHtml(reply: unknown): string {
  if (!reply || typeof reply !== 'object') return '';
  const rec = reply as { status?: unknown; content_b64?: unknown };
  if (rec.status !== 'sketch' || typeof rec.content_b64 !== 'string' || !rec.content_b64) return '';
  return decodeBase64(rec.content_b64);
}

function decodeBase64(raw: string): string {
  try {
    const binary = atob(raw);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
    const html = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    return html.trim() ? html : '';
  } catch {
    return '';
  }
}

function mountSlot(anchor: Element, html: string, pageUrl: string, session: Session, gen: number) {
  if (session.gen !== gen || !anchor.isConnected) return;
  const slotEl = document.createElement('div');
  slotEl.className = 'board-sketch-slot';
  anchor.insertAdjacentElement('afterend', slotEl);
  if (session.gen !== gen) {
    slotEl.remove();
    return;
  }
  const root: Root = createRoot(slotEl);
  const paint = (appearance: SketchAppearance) => {
    flushSync(() => {
      root.render(createElement(SketchPreview, { html, pageUrl, appearance }));
    });
  };
  const slot: Slot = {
    paint,
    unmount: () => {
      flushSync(() => root.unmount());
      slotEl.remove();
    },
  };
  session.slots.push(slot);
  if (session.gen !== gen) {
    slot.unmount();
    session.slots = session.slots.filter((item) => item !== slot);
    return;
  }
  paint(currentAppearance());
}

async function attach(anchor: Element, session: Session, gen: number, read: Reader) {
  const pageUrl = anchor.getAttribute('href') || '';
  if (!isBoardPage(pageUrl)) return;
  let reply: unknown;
  try {
    reply = await read(pageUrl);
  } catch {
    return;
  }
  const html = sketchHtml(reply);
  if (!html) return;
  mountSlot(anchor, html, pageUrl, session, gen);
}

export async function mountBoardSketches(
  container: Element | null | undefined,
  read: Reader = fetchBoardSketch,
) {
  if (!container) return;
  const session = ensure(container);
  session.read = read;
  session.gen += 1;
  clearSlots(session);
  const gen = session.gen;
  watchTheme();
  const links = [...container.querySelectorAll('a[href]')];
  await Promise.all(links.map((anchor) => attach(anchor, session, gen, read)));
}
