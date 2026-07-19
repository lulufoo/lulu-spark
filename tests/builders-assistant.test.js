// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const renderFeedMock = vi.fn();

vi.mock('../frontend/js/feed.js', () => ({
  renderFeed: (...args) => renderFeedMock(...args),
}));

import { mountBuildersAssistantWidget } from '../frontend/js/builders-assistant.js';

describe('mountBuildersAssistantWidget open/close + renderFeed mount', () => {
  let anchor;

  beforeEach(() => {
    anchor = document.createElement('div');
    document.body.appendChild(anchor);
    renderFeedMock.mockReset();
    renderFeedMock.mockImplementation((container) => {
      container.innerHTML = '<div class="feed-mock">feed</div>';
    });
  });

  afterEach(() => {
    document.querySelectorAll('.builders-entry, .builders-modal-host').forEach((el) => el.remove());
    anchor.remove();
  });

  it('setOpen(true) shows host and calls renderFeed once on builders-modal-body only', () => {
    const { setOpen, host, body, dispose } = mountBuildersAssistantWidget(anchor);

    expect(host.hidden).toBe(true);
    setOpen(true);

    expect(host.hidden).toBe(false);
    expect(renderFeedMock).toHaveBeenCalledTimes(1);
    expect(renderFeedMock).toHaveBeenCalledWith(body);
    expect(body.classList.contains('builders-modal-body')).toBe(true);
    expect(renderFeedMock.mock.calls[0][0]).not.toBe(host);

    // chrome must survive renderFeed's innerHTML wipe of the body container
    expect(host.querySelector('.builders-modal-header')).not.toBeNull();
    expect(host.querySelector('[aria-label="Close"]')).not.toBeNull();
    expect(body.querySelector('.feed-mock')).not.toBeNull();

    dispose();
  });

  it('header × (aria-label="Close") closes: host hidden, DOM retained', () => {
    const { setOpen, host, close, dispose } = mountBuildersAssistantWidget(anchor);
    expect(typeof setOpen).toBe('function');
    expect(typeof close).toBe('function');

    setOpen(true);
    expect(host.hidden).toBe(false);

    const closeBtn = host.querySelector('[aria-label="Close"]');
    expect(closeBtn).not.toBeNull();
    closeBtn.click();

    expect(host.hidden).toBe(true);
    expect(anchor.contains(host)).toBe(true);
    expect(host.querySelector('.builders-modal-header')).not.toBeNull();
    expect(host.querySelector('.builders-modal-body')).not.toBeNull();
    expect(document.querySelectorAll('.builders-modal-host')).toHaveLength(1);

    dispose();
  });

  it('reopen refreshes via renderFeed again without a second feed host', () => {
    const { setOpen, host, body, dispose } = mountBuildersAssistantWidget(anchor);

    setOpen(true);
    host.querySelector('[aria-label="Close"]').click();
    expect(host.hidden).toBe(true);

    setOpen(true);
    expect(host.hidden).toBe(false);
    expect(renderFeedMock).toHaveBeenCalledTimes(2);
    expect(renderFeedMock).toHaveBeenNthCalledWith(1, body);
    expect(renderFeedMock).toHaveBeenNthCalledWith(2, body);
    expect(document.querySelectorAll('.builders-modal-host')).toHaveLength(1);
    expect(document.querySelectorAll('.builders-modal-body')).toHaveLength(1);

    dispose();
  });

  it('close leaves nodes in DOM (hidden), does not remove them', () => {
    const { setOpen, close, host, entry, dispose } = mountBuildersAssistantWidget(anchor);
    setOpen(true);
    close();

    expect(host.hidden).toBe(true);
    expect(anchor.contains(host)).toBe(true);
    expect(anchor.contains(entry)).toBe(true);
    expect(host.isConnected).toBe(true);

    dispose();
  });

  it('optional: outside click (capture-phase) closes like ×', () => {
    const { setOpen, host, dispose } = mountBuildersAssistantWidget(anchor);
    setOpen(true);
    expect(host.hidden).toBe(false);

    const outside = document.createElement('div');
    document.body.appendChild(outside);
    outside.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));

    expect(host.hidden).toBe(true);
    expect(anchor.contains(host)).toBe(true);

    outside.remove();
    dispose();
  });

  it('FAB open path shows host; renderFeed empty/error still allows × close', async () => {
    renderFeedMock.mockImplementation((container) => {
      container.innerHTML = '<div class="feed-error">加载失败：network</div>';
    });

    const { host, fab, dispose } = mountBuildersAssistantWidget(anchor);
    fab.click();

    expect(host.hidden).toBe(false);
    expect(renderFeedMock).toHaveBeenCalledTimes(1);
    expect(host.querySelector('.builders-modal-body .feed-error')).not.toBeNull();
    expect(host.querySelector('.builders-modal-header')).not.toBeNull();

    host.querySelector('[aria-label="Close"]').click();
    expect(host.hidden).toBe(true);
    expect(anchor.contains(host)).toBe(true);

    dispose();
  });

  it('fails the contract if renderFeed were bound to outer host (chrome wipe guard)', () => {
    const { setOpen, host, body, dispose } = mountBuildersAssistantWidget(anchor);
    setOpen(true);

    const mountTarget = renderFeedMock.mock.calls[0][0];
    expect(mountTarget).toBe(body);
    expect(mountTarget.classList.contains('builders-modal-body')).toBe(true);
    expect(mountTarget).not.toBe(host);
    expect(host.querySelector('.builders-modal-header')).not.toBeNull();

    dispose();
  });
});
