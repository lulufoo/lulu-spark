// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { initTooltip } from '../../frontend/src/app-shell/ui/tooltip.ts';

describe('workspace "…" mark uses the fast tooltip', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    document.body.innerHTML =
      '<button class="home-chat-nav-item"><span class="home-nav-hidden-count" data-tip="12 projects collapsed">…</span></button>';
    initTooltip();
  });

  afterEach(() => {
    vi.useRealTimers();
    document.body.innerHTML = '';
  });

  it('shows the text 150ms after the pointer enters, far sooner than the native title delay', () => {
    const mark = document.querySelector('.home-nav-hidden-count');
    mark.dispatchEvent(new MouseEvent('mouseover', { bubbles: true, clientX: 20, clientY: 20 }));
    const tip = document.getElementById('_tip');
    expect(tip.classList.contains('visible')).toBe(false);
    vi.advanceTimersByTime(150);
    expect(tip.classList.contains('visible')).toBe(true);
    expect(tip.textContent).toBe('12 projects collapsed');
    mark.dispatchEvent(new MouseEvent('mouseout', { bubbles: true, relatedTarget: document.body }));
    expect(tip.classList.contains('visible')).toBe(false);
  });
});
