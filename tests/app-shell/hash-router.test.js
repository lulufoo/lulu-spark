// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

describe('mountHashRouter', () => {
  beforeEach(async () => {
    const { unmountHashRouter } = await import('../../frontend/src/hash-router.tsx');
    unmountHashRouter();
    document.body.innerHTML = '';
    window.location.hash = '';
  });

  afterEach(async () => {
    const { unmountHashRouter } = await import('../../frontend/src/hash-router.tsx');
    unmountHashRouter();
    window.location.hash = '';
  });

  it('dispatches the matching vanilla island handler for #/home', async () => {
    const { mountHashRouter } = await import('../../frontend/src/hash-router.tsx');
    const home = vi.fn();
    const workbench = vi.fn();
    window.location.hash = '#/home';
    mountHashRouter({ home, workbench }, { fallback: '#/home' });
    await vi.waitFor(() => {
      expect(home).toHaveBeenCalled();
    });
    expect(workbench).not.toHaveBeenCalled();
    const ctx = home.mock.calls[0][0];
    expect(ctx.name).toBe('home');
  });

  it.each(['', '#/no-such-route'])(
    'hash %j replaces to fallback and mounts that handler',
    async (startHash) => {
      const { mountHashRouter } = await import('../../frontend/src/hash-router.tsx');
      const home = vi.fn();
      window.location.hash = startHash;
      mountHashRouter({ home }, { fallback: '#/home' });
      await vi.waitFor(() => {
        expect(window.location.hash).toBe('#/home');
        expect(home).toHaveBeenCalled();
      });
    },
  );

  it('hashchange dispatches the new route handler', async () => {
    const { mountHashRouter } = await import('../../frontend/src/hash-router.tsx');
    const home = vi.fn();
    const workbench = vi.fn();
    const readLater = vi.fn();
    window.location.hash = '#/home';
    mountHashRouter({ home, workbench, 'read-later': readLater }, { fallback: '#/home' });
    await vi.waitFor(() => {
      expect(home).toHaveBeenCalled();
    });

    window.location.hash = '#/workbench?date=20260828&note=inbox/a.md';
    await vi.waitFor(() => {
      expect(workbench).toHaveBeenCalled();
    });
    expect(workbench.mock.calls[0][0]).toEqual({
      name: 'workbench',
      params: { date: '20260828', note: 'inbox/a.md' },
    });

    window.location.hash = '#/read-later';
    await vi.waitFor(() => {
      expect(readLater).toHaveBeenCalled();
    });
    expect(readLater.mock.calls[0][0]).toEqual({ name: 'read-later', params: {} });
  });

  it('does not replace a valid #/workbench query to fallback', async () => {
    const { mountHashRouter } = await import('../../frontend/src/hash-router.tsx');
    const home = vi.fn();
    const workbench = vi.fn();
    const start = '#/workbench?date=20260719&note=inbox/notes/x.md&layer=raw';
    window.location.hash = start;
    mountHashRouter({ home, workbench }, { fallback: '#/home' });
    await vi.waitFor(() => {
      expect(workbench).toHaveBeenCalled();
    });
    expect(window.location.hash).toBe(start);
    expect(home).not.toHaveBeenCalled();
  });

  it('creates a hidden #react-root host', async () => {
    const { mountHashRouter } = await import('../../frontend/src/hash-router.tsx');
    window.location.hash = '#/home';
    mountHashRouter({ home: vi.fn() }, { fallback: '#/home' });
    await vi.waitFor(() => {
      const host = document.getElementById('react-root');
      expect(host).toBeTruthy();
      expect(host.hidden).toBe(true);
    });
  });
});
