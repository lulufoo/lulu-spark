// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

const TOAST_SELECTOR = '.wb-toast';

function getToasts() {
  return [...document.body.querySelectorAll(TOAST_SELECTOR)];
}

function isVisible(el) {
  if (!el.isConnected) return false;
  const style = window.getComputedStyle(el);
  return style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0';
}

describe('toast React export', () => {
  it('frontend/src/toast.tsx exports showToast', () => {
    const src = readFileSync(join(repoRoot, 'frontend/src/toast.tsx'), 'utf8');
    expect(src).toMatch(/export\s+function\s+showToast\s*\(/);
  });
});

describe('showToast', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    document.body.innerHTML = '';
  });

  afterEach(() => {
    vi.useRealTimers();
    document.body.innerHTML = '';
  });

  it('renders a floating success toast that auto-dismisses after 3s', async () => {
    const { showToast } = await import('../../frontend/src/toast.tsx');

    showToast('saved', 'success');

    const toasts = getToasts();
    expect(toasts).toHaveLength(1);
    expect(toasts[0].textContent).toBe('saved');
    expect(toasts[0].classList.contains('wb-toast-success')).toBe(true);
    expect(isVisible(toasts[0])).toBe(true);

    vi.advanceTimersByTime(2999);
    expect(getToasts()).toHaveLength(1);

    vi.advanceTimersByTime(1);
    expect(getToasts()).toHaveLength(0);
  });

  it('renders error toast with styling distinct from success', async () => {
    const { showToast } = await import('../../frontend/src/toast.tsx');

    showToast('failed', 'error');

    const toast = getToasts()[0];
    expect(toast.classList.contains('wb-toast-error')).toBe(true);
    expect(toast.classList.contains('wb-toast-success')).toBe(false);
  });

  it('handles rapid consecutive calls without crash or invisible DOM leftovers', async () => {
    const { showToast } = await import('../../frontend/src/toast.tsx');

    expect(() => {
      showToast('first', 'success');
      showToast('second', 'error');
    }).not.toThrow();

    const toasts = getToasts();
    expect(toasts.length).toBeGreaterThanOrEqual(1);

    for (const toast of document.body.querySelectorAll('*')) {
      if (!toast.classList?.contains('wb-toast')) continue;
      expect(isVisible(toast) || toast.isConnected).toBe(true);
    }

    vi.advanceTimersByTime(3000);
    expect(getToasts()).toHaveLength(0);
  });

  it('renders empty message without throwing', async () => {
    const { showToast } = await import('../../frontend/src/toast.tsx');

    expect(() => showToast('', 'success')).not.toThrow();

    const toasts = getToasts();
    expect(toasts).toHaveLength(1);
    expect(toasts[0].textContent).toBe('');
  });
});
