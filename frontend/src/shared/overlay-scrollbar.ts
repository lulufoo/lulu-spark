/**
 * Draw the themed sidebar scrollbar as app pixels.
 *
 * WKWebView paints scroll lists with the native overlay thumb in system gray and
 * ignores both the styled `::-webkit-scrollbar` parts and `scrollbar-color`, so a
 * scrollbar in the row-hover purple has to be drawn here. Like the native overlay
 * bar, the thumb stays hidden until the list scrolls and fades out afterwards;
 * the native bar itself is hidden per scroller in CSS.
 */

/** Idle time after the last scroll event before the thumb fades out. */
const SCROLL_HIDE_DELAY_MS = 700;
const MIN_THUMB_HEIGHT = 28;
/** Breathing room above and below the thumb inside the track. */
const TRACK_INSET = 2;

export function attachOverlayScrollbar(scroller: HTMLElement): () => void {
  const host = scroller.parentElement;
  if (!host) return () => {};

  // The track anchors on the host, so the host has to be the containing block.
  let hostWasStatic = false;
  if (getComputedStyle(host).position === 'static') {
    hostWasStatic = true;
    host.style.position = 'relative';
  }

  const track = document.createElement('div');
  track.className = 'overlay-scrollbar-track';
  track.setAttribute('aria-hidden', 'true');
  const thumb = document.createElement('div');
  thumb.className = 'overlay-scrollbar-thumb';
  track.appendChild(thumb);
  host.appendChild(track);

  let hideTimer: ReturnType<typeof setTimeout> | null = null;

  const update = () => {
    // Anchor the track on the scroller's own box, so the thumb hugs the scroller's
    // right edge wherever the scroller sits inside its parent.
    const box = scroller.getBoundingClientRect();
    const home = host.getBoundingClientRect();
    track.style.left = `${Math.round(box.left - home.left)}px`;
    track.style.top = `${Math.round(box.top - home.top)}px`;
    track.style.width = `${Math.round(box.width)}px`;
    track.style.height = `${Math.round(box.height)}px`;
    const overflow = scroller.scrollHeight - scroller.clientHeight;
    if (overflow <= 1 || box.height <= 0 || box.width <= 0) {
      track.style.display = 'none';
      return;
    }
    track.style.display = 'block';
    const trackHeight = scroller.clientHeight - 2 * TRACK_INSET;
    const thumbHeight = Math.max(MIN_THUMB_HEIGHT, Math.round((scroller.clientHeight / scroller.scrollHeight) * trackHeight));
    const maxThumbOffset = trackHeight - thumbHeight;
    const thumbOffset = Math.round((scroller.scrollTop / overflow) * maxThumbOffset);
    thumb.style.top = `${TRACK_INSET + thumbOffset}px`;
    thumb.style.height = `${thumbHeight}px`;
  };

  const onScroll = () => {
    const wasHidden = track.style.display === 'none';
    update();
    if (track.style.display === 'none') return;
    if (hideTimer) clearTimeout(hideTimer);
    // Reveal instantly on a fresh scroll; only the fade-out is animated.
    if (wasHidden || track.style.opacity !== '1') track.style.transition = 'none';
    track.style.opacity = '1';
    void track.offsetHeight;
    track.style.transition = '';
    hideTimer = setTimeout(() => {
      track.style.opacity = '0';
      hideTimer = null;
    }, SCROLL_HIDE_DELAY_MS);
  };

  scroller.addEventListener('scroll', onScroll, { passive: true });
  // List rows mount, grow, and unmount with the data; re-measure whenever the
  // scroller's box or its content does.
  let contentObserver: MutationObserver | null = null;
  let boxObserver: ResizeObserver | null = null;
  if (typeof ResizeObserver === 'function') {
    boxObserver = new ResizeObserver(update);
    boxObserver.observe(scroller);
  }
  if (typeof MutationObserver === 'function') {
    contentObserver = new MutationObserver(update);
    contentObserver.observe(scroller, { childList: true, subtree: true });
  }
  update();

  return () => {
    scroller.removeEventListener('scroll', onScroll);
    if (hideTimer) clearTimeout(hideTimer);
    boxObserver?.disconnect();
    contentObserver?.disconnect();
    track.remove();
    if (hostWasStatic) host.style.position = '';
  };
}
