import type { SketchAttachment } from '../state/sketch-attachment.ts';
import type { ReplyOrigin } from '../state/sketch-freshness.ts';
import { parseSketchReply } from '../state/sketch-reply.ts';
import { invalidateSketchAttachments, type SketchEventPool } from './sketch-events.ts';
import { elapsedMs, logSketch } from './sketch-log.ts';

export type SketchReader = (url: string) => Promise<unknown>;

async function readSource(
  attachment: SketchAttachment,
  reader: SketchReader,
  origin: ReplyOrigin,
  pool: SketchEventPool,
) {
  const started = performance.now();
  try {
    const reply = await reader(attachment.pageUrl);
    if (!attachment.isLive() || (origin === 'cache' && attachment.remoteTrusted)) return;
    const value = reply as { status?: unknown; content_b64?: unknown } | null;
    const params = {
      diagram_id: attachment.diagramId, origin, ms: elapsedMs(started),
      status: value?.status,
      ...(typeof value?.content_b64 === 'string' ? { bytes: value.content_b64.length } : {}),
    };
    logSketch(origin === 'cache' ? 'cache.read' : 'remote.read', params);
    if (origin === 'remote') logSketch('sketch.host_reply', params);
    const parsed = parseSketchReply(reply, attachment.diagramId);
    if (parsed.kind === 'source') {
      if (pool.freshness.acceptSource(parsed.source.identity, origin, parsed.source.version)) {
        attachment.accept(parsed.source, origin);
      }
    } else if (parsed.kind === 'invalidation') {
      if (parsed.event) {
        if (!invalidateSketchAttachments(pool, parsed.event, origin)) return;
        // A first reply can identify a denial before this attachment has an
        // identity to match in the fanout above.
        if (!attachment.identity) attachment.fallback();
      } else attachment.invalidate();
      if (origin === 'remote') attachment.remoteTrusted = true;
    } else if (origin === 'remote' && attachment.view.mode === 'loading') attachment.fallback();
  } catch {
    if (!attachment.isLive()) return;
    if (origin === 'remote' && attachment.view.mode === 'loading') attachment.fallback();
    logSketch(origin === 'cache' ? 'cache.read' : 'remote.read', {
      diagram_id: attachment.diagramId, ms: elapsedMs(started), status: 'error',
    }, 'warn');
  }
}

export async function readSketchAttachment(
  attachment: SketchAttachment,
  cached: SketchReader,
  remote: SketchReader,
  pool: SketchEventPool,
) {
  logSketch('sketch.mount', { diagram_id: attachment.diagramId });
  // Neither a hanging cache read nor a hanging remote delays the other path.
  await Promise.all([
    readSource(attachment, cached, 'cache', pool),
    readSource(attachment, remote, 'remote', pool),
  ]);
  if (attachment.isLive() && attachment.view.mode === 'loading') attachment.fallback();
}
