import type { BoardReadIdentity } from '../../host/api/board-sketch.ts';

export type SketchSource = {
  status: 'sketch' | 'not_sketch' | 'invalid_source';
  identity: BoardReadIdentity;
  version: number;
  html: string;
  title: string;
};

export type SketchView = {
  mode: 'loading' | 'preview' | 'link';
  html: string;
  title: string;
  revision: number;
  otherFormat: boolean;
};

/** One Markdown link stays attached even while its current source is a link. */
export class SketchAttachment {
  identity?: BoardReadIdentity;
  version?: number;
  remoteTrusted = false;
  private active = true;
  private listeners = new Set<() => void>();
  readonly view: SketchView;

  constructor(
    readonly anchor: Element,
    readonly pageUrl: string,
    readonly diagramId: string,
    private readonly isCurrent: () => boolean,
  ) {
    this.view = {
      mode: 'loading', html: '', revision: 0, otherFormat: false,
      title: (anchor.textContent || '').replace(/\s+/g, ' ').trim(),
    };
  }

  isLive(): boolean {
    return this.active && this.isCurrent() && this.anchor.isConnected;
  }

  subscribe(listener: () => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  accept(source: SketchSource, origin: 'cache' | 'remote') {
    const previous = this.identity;
    this.identity = source.identity;
    if (origin === 'remote') this.remoteTrusted = true;
    // Freshness changes do not change the content. Keep the iframe and title
    // when the current credential's source version is unchanged.
    if (this.view.mode === 'preview' && this.version === source.version
      && previous?.credential_scope === source.identity.credential_scope) return;
    this.version = source.version;
    if (source.status !== 'sketch') {
      this.fallback(source.status === 'not_sketch');
      return;
    }
    if (this.view.mode === 'preview') this.view.revision += 1;
    this.view.mode = 'preview';
    this.view.html = source.html;
    this.view.title = source.title;
    this.view.otherFormat = false;
    this.notify();
  }

  invalidate(identity?: BoardReadIdentity) {
    if (identity) this.identity = identity;
    this.version = undefined;
    this.fallback();
  }

  fallback(otherFormat = false) {
    this.view.mode = 'link';
    this.view.html = '';
    this.view.otherFormat = otherFormat;
    this.notify();
  }

  dispose() {
    this.active = false;
    this.fallback();
    this.listeners.clear();
  }

  private notify() {
    for (const listener of this.listeners) listener();
  }
}
