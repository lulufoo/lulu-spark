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
  const sheet = document.getElementById('theme-sheet');
  if (sheet instanceof HTMLLinkElement) {
    const href = sheet.getAttribute('href') || sheet.href || '';
    const id = sheet.dataset.themeId;
    if (id === 'dark' || href.includes('theme-dark')) return 'dark';
    if (id === 'light' || href.includes('theme-light')) return 'light';
  }
  const theme = document.documentElement.dataset.theme;
  if (theme === 'dark' || theme === 'light') return theme;
  return 'light';
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
  for (const note of session.container.querySelectorAll('.board-sketch-note')) note.remove();
}

export function releaseBoardSketches(container: Element | null | undefined) {
  if (!container) return;
  const session = sessions.get(container);
  if (!session) return;
  session.gen += 1;
  clearSlots(session);
  sessions.delete(container);
  live.delete(session);
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

function paintLive() {
  const appearance = currentAppearance();
  for (const session of live) {
    for (const slot of session.slots) slot.paint(appearance);
  }
}

function watchTheme() {
  if (watchingTheme) return;
  watchingTheme = true;
  document.addEventListener('spark-theme', paintLive);
  if (typeof MutationObserver !== 'function') return;
  const observer = new MutationObserver(paintLive);
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme'],
  });
  const sheet = document.getElementById('theme-sheet');
  if (sheet) {
    observer.observe(sheet, { attributes: true, attributeFilter: ['href', 'data-theme-id'] });
  }
}

function sketchView(reply: unknown): { html: string; title: string } | null {
  if (!reply || typeof reply !== 'object') return null;
  const rec = reply as { status?: unknown; content_b64?: unknown; title?: unknown };
  if (rec.status !== 'sketch' || typeof rec.content_b64 !== 'string' || !rec.content_b64) return null;
  const html = decodeBase64(rec.content_b64);
  if (!html) return null;
  const title = typeof rec.title === 'string' ? rec.title.trim() : '';
  return { html, title };
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

function mountSlot(
  anchor: Element,
  html: string,
  title: string,
  pageUrl: string,
  session: Session,
  gen: number,
) {
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
      root.render(createElement(SketchPreview, { html, pageUrl, appearance, title }));
    });
  };
  if (anchor instanceof HTMLElement) anchor.hidden = true;
  const slot: Slot = {
    paint,
    unmount: () => {
      flushSync(() => root.unmount());
      slotEl.remove();
      if (anchor instanceof HTMLElement) anchor.hidden = false;
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
  if (isRecord(reply) && reply.status === 'not_sketch') {
    markOtherFormat(anchor, session, gen);
    return;
  }
  const view = sketchView(reply);
  if (!view) return;
  mountSlot(anchor, view.html, view.title, pageUrl, session, gen);
}

function isRecord(reply: unknown): reply is { status?: unknown } {
  return !!reply && typeof reply === 'object';
}

function markOtherFormat(anchor: Element, session: Session, gen: number) {
  if (session.gen !== gen || !anchor.isConnected) return;
  const note = document.createElement('span');
  note.className = 'board-sketch-note';
  note.textContent = 'Other Board formats stay as links';
  anchor.insertAdjacentElement('afterend', note);
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
