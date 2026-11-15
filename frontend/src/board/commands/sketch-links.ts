import {
  fetchBoardSketch, fetchCachedBoardSketch, listenBoardCacheInvalidated,
} from '../../host/api/board-sketch.ts';
import { SketchAttachment } from '../state/sketch-attachment.ts';
import { paintSketchAttachment } from '../ui/sketch-slot.tsx';
import { acquireSketchEvents, type InvalidationListener } from './sketch-events.ts';
import { boardDiagramId, warmSandbox } from './sketch-preview.ts';
import { readSketchAttachment, type SketchReader } from './sketch-read.ts';

export type SketchReaders = {
  readCached: SketchReader;
  readRemote: SketchReader;
  listenInvalidated: InvalidationListener;
};

type Session = {
  gen: number;
  container: Element;
  attachments: SketchAttachment[];
  paints: (() => void)[];
  readers: SketchReaders;
  releaseEvents?: () => void;
};

const defaults: SketchReaders = {
  readCached: fetchCachedBoardSketch,
  readRemote: fetchBoardSketch,
  listenInvalidated: listenBoardCacheInvalidated,
};
const noEvents: InvalidationListener = async () => () => {};
const sessions = new WeakMap<Element, Session>();
const live = new Set<Session>();

function clearAttachments(session: Session) {
  session.gen += 1;
  for (const attachment of session.attachments) attachment.dispose();
  for (const stopPaint of session.paints) stopPaint();
  session.attachments = [];
  session.paints = [];
  session.releaseEvents?.();
  session.releaseEvents = undefined;
}

export function releaseBoardSketches(container: Element | null | undefined) {
  if (!container) return;
  const session = sessions.get(container);
  if (!session) return;
  clearAttachments(session);
  sessions.delete(container);
  live.delete(session);
}

/** A Settings save/remove starts one fresh pair of reads. Host access events
 * revoke matching attachments without starting another read loop. */
export function invalidateBoardSketches() {
  for (const session of [...live]) {
    if (!session.container.isConnected) releaseBoardSketches(session.container);
    else void mountBoardSketches(session.container, session.readers);
  }
}

function resolveReaders(readers: SketchReaders | SketchReader): SketchReaders {
  // A custom single-reader transport is remote-only. Supplying the pair opts
  // into cache reads and its matching invalidation subscription.
  return typeof readers === 'function'
    ? { readCached: async () => ({ status: 'cache_miss' }), readRemote: readers, listenInvalidated: noEvents }
    : readers;
}

export async function mountBoardSketches(
  container: Element | null | undefined,
  readers: SketchReaders | SketchReader = defaults,
) {
  if (!container) return;
  for (const session of [...live]) {
    if (!session.container.isConnected) releaseBoardSketches(session.container);
  }
  warmSandbox();
  let session = sessions.get(container);
  if (session) clearAttachments(session);
  else {
    session = { gen: 0, container, attachments: [], paints: [], readers: defaults };
    sessions.set(container, session);
    live.add(session);
  }
  const owner = session;
  const gen = owner.gen;
  owner.readers = resolveReaders(readers);
  for (const anchor of container.querySelectorAll('a[href]')) {
    const pageUrl = anchor.getAttribute('href') || '';
    const diagramId = boardDiagramId(pageUrl);
    if (!diagramId) continue;
    const attachment = new SketchAttachment(anchor, pageUrl, diagramId, () => owner.gen === gen);
    owner.attachments.push(attachment);
    owner.paints.push(paintSketchAttachment(attachment));
  }
  if (!owner.attachments.length) return;
  const events = acquireSketchEvents(owner.readers.listenInvalidated, owner);
  owner.releaseEvents = events.release;
  await events.ready;
  if (owner.gen !== gen || !container.isConnected) return;
  await Promise.all(owner.attachments.map((attachment) => readSketchAttachment(
    attachment, owner.readers.readCached, owner.readers.readRemote, events.pool,
  )));
}
