// @vitest-environment jsdom
/**
 * T6: Builders is a content adapter — renderFeed into the shell slot; no modal chrome.
 */
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const renderFeedMock = vi.fn();

vi.mock('../frontend/js/builders/feed.js', () => ({
  renderFeed: (...args) => renderFeedMock(...args),
}));

import { createBuildersContentAdapter } from '../frontend/js/builders/assistant.js';

describe('createBuildersContentAdapter · slot mount + renderFeed', () => {
  /** @type {HTMLElement} */
  let slot;

  beforeEach(() => {
    slot = document.createElement('div');
    document.body.appendChild(slot);
    renderFeedMock.mockReset();
    renderFeedMock.mockImplementation((container) => {
      container.innerHTML = '<div class="feed-mock">feed</div>';
    });
  });

  afterEach(() => {
    slot.remove();
  });

  it('mount calls renderFeed once on the content slot (no modal host/body)', () => {
    const handle = createBuildersContentAdapter().mount(slot, { host: {} });

    expect(renderFeedMock).toHaveBeenCalledTimes(1);
    expect(renderFeedMock).toHaveBeenCalledWith(slot);
    expect(slot.querySelector('.feed-mock')).not.toBeNull();
    expect(slot.querySelector('.builders-modal-host')).toBeNull();
    expect(slot.querySelector('.builders-modal-header')).toBeNull();
    expect(document.querySelector('.builders-entry-fab')).toBeNull();

    handle.unmount();
  });

  it('unmount clears slot content', () => {
    const handle = createBuildersContentAdapter().mount(slot, { host: {} });
    expect(slot.querySelector('.feed-mock')).not.toBeNull();
    handle.unmount();
    expect(slot.innerHTML).toBe('');
  });

  it('remount refreshes via renderFeed again on the same slot', () => {
    const adapter = createBuildersContentAdapter();
    const first = adapter.mount(slot, { host: {} });
    first.unmount();
    const second = adapter.mount(slot, { host: {} });

    expect(renderFeedMock).toHaveBeenCalledTimes(2);
    expect(renderFeedMock).toHaveBeenNthCalledWith(1, slot);
    expect(renderFeedMock).toHaveBeenNthCalledWith(2, slot);
    expect(slot.querySelector('.feed-mock')).not.toBeNull();

    second.unmount();
  });

  it('renderFeed empty/error still paints into the slot (chrome is shell-owned)', () => {
    renderFeedMock.mockImplementation((container) => {
      container.innerHTML = '<div class="feed-error">加载失败：network</div>';
    });

    const handle = createBuildersContentAdapter().mount(slot, { host: {} });
    expect(slot.querySelector('.feed-error')).not.toBeNull();
    expect(slot.querySelector('.builders-modal-header')).toBeNull();
    handle.unmount();
  });
});
