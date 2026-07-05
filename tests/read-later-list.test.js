// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const invokeMock = vi.fn();
const getJsonMock = vi.fn();

vi.mock('../frontend/js/apiClient.js', async (importOriginal) => {
  const actual = await importOriginal();
  return {
    ...actual,
    resolveReadDriver: vi.fn(() => ({ getJson: getJsonMock })),
    createApiClient: (driver) => ({
      getJson: driver?.getJson ?? getJsonMock,
    }),
  };
});

import { parseHash } from '../frontend/js/router/index.js';
import {
  bindFocusRefresh,
  loadReadLaterEntries,
  markEntryRead,
  mountReadLaterList,
  renderUnavailableState,
} from '../frontend/js/components/read-later-list.js';

const UNAVAILABLE_MSG = '列表暂时不可用，请稍后重试';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const mainJs = readFileSync(join(fixtureRoot, 'frontend/js/main.js'), 'utf8');
const indexHtml = readFileSync(join(fixtureRoot, 'frontend/index.html'), 'utf8');

function extractFunctionBody(source, name) {
  const start = source.indexOf(`function ${name}`);
  if (start === -1) return '';
  const braceStart = source.indexOf('{', start);
  let depth = 0;
  for (let i = braceStart; i < source.length; i += 1) {
    if (source[i] === '{') depth += 1;
    if (source[i] === '}') {
      depth -= 1;
      if (depth === 0) return source.slice(braceStart, i + 1);
    }
  }
  return '';
}

function seedReadLaterRouteDom({ includeReadLaterView = true } = {}) {
  const readLaterView = includeReadLaterView
    ? '<div id="read-later-view" style="display:none;"></div>'
    : '';
  document.body.innerHTML = `
    <div id="home-view" style="display:block;"></div>
    <div id="corpus-doc-view" style="display:block;"></div>
    <div class="layout" style="display:block;"></div>
    ${readLaterView}
    <button id="btn-feed"></button>
    <div id="feed-view" style="display:block;"></div>
  `;
}

function createMountReadLaterRouteFromMain() {
  const body = extractFunctionBody(mainJs, 'mountReadLaterRoute');
  if (!body) return null;
  return new Function(
    'unmountHomeHub',
    'unmountCorpusDocList',
    'unmountReadLaterList',
    'feedView',
    'mountReadLaterList',
    'hideHomeView',
    'hideCorpusDocView',
    `return function mountReadLaterRoute() ${body}`,
  );
}

const sampleEntries = [
  {
    id: 'abc123',
    url: 'https://example.com/unread',
    title: 'Unread Article',
    saved_at: '2026-07-01T10:00:00Z',
    read: false,
  },
  {
    id: 'def456',
    url: 'https://example.com/read',
    title: 'Read Article',
    saved_at: '2026-07-01T09:00:00Z',
    read: true,
  },
];

describe('read-later route', () => {
  describe('source wiring', () => {
    it('index.html includes #read-later-view shell', () => {
      expect(indexHtml).toMatch(/id="read-later-view"/);
    });

    it('main.js defines mountReadLaterRoute', () => {
      expect(mainJs).toMatch(/function mountReadLaterRoute/);
    });

    it('main.js registers read-later via wrapRouteMount in initRouter', () => {
      expect(mainJs).toMatch(
        /['"]read-later['"]:\s*wrapRouteMount\s*\(\s*['"]read-later['"]\s*,\s*mountReadLaterRoute/,
      );
    });

    it('mountReadLaterRoute calls mountReadLaterList', () => {
      const body = extractFunctionBody(mainJs, 'mountReadLaterRoute');
      expect(body).toMatch(/mountReadLaterList\s*\(/);
    });
  });

  describe('parseHash', () => {
    it('returns read-later for #/read-later', () => {
      expect(parseHash('#/read-later')).toEqual({ name: 'read-later', params: {} });
    });

    it('returns read-later for #/read-later/ with trailing slash', () => {
      expect(parseHash('#/read-later/')).toEqual({ name: 'read-later', params: {} });
    });
  });

  describe('mountReadLaterRoute view visibility', () => {
    let routeUnmount;

    beforeEach(() => {
      seedReadLaterRouteDom();
      routeUnmount = null;
      getJsonMock.mockResolvedValue([sampleEntries[0]]);
    });

    afterEach(() => {
      routeUnmount?.();
      routeUnmount = null;
    });

    function invokeMountReadLaterRoute() {
      const factory = createMountReadLaterRouteFromMain();
      expect(factory).not.toBeNull();
      const mountWithCapture = (container) => {
        const handle = mountReadLaterList(container);
        routeUnmount = handle.dispose;
        return { ...handle, unmount: handle.dispose };
      };
      const mountReadLaterRoute = factory(
        null,
        null,
        null,
        document.getElementById('feed-view'),
        mountWithCapture,
        () => {
          const homeView = document.getElementById('home-view');
          if (homeView) homeView.style.display = 'none';
        },
        () => {
          const docView = document.getElementById('corpus-doc-view');
          if (docView) docView.style.display = 'none';
          const layout = document.querySelector('.layout');
          if (layout) layout.style.display = 'none';
        },
      );
      mountReadLaterRoute();
    }

    it('shows read-later-view and hides other top-level views', async () => {
      await invokeMountReadLaterRoute();

      expect(document.getElementById('read-later-view').style.display).not.toBe('none');
      expect(document.getElementById('home-view').style.display).toBe('none');
      expect(document.getElementById('corpus-doc-view').style.display).toBe('none');
      expect(document.querySelector('.layout').style.display).toBe('none');
    });

    it('mounts list into read-later-view with mock data', async () => {
      await invokeMountReadLaterRoute();

      const container = document.getElementById('read-later-view');
      await vi.waitFor(() => {
        expect(container.querySelector('.read-later-item')).not.toBeNull();
      });
    });

    it('does not throw when read-later-view DOM is missing', () => {
      seedReadLaterRouteDom({ includeReadLaterView: false });
      vi.spyOn(console, 'warn').mockImplementation(() => {});

      expect(() => invokeMountReadLaterRoute()).not.toThrow();
    });
  });
});

describe('loadReadLaterEntries', () => {
  beforeEach(() => {
    getJsonMock.mockReset();
  });

  it('GET /api/read-later via apiClient and returns entries array', async () => {
    getJsonMock.mockResolvedValue(sampleEntries);
    const entries = await loadReadLaterEntries();
    expect(getJsonMock).toHaveBeenCalledWith('/api/read-later');
    expect(entries).toEqual(sampleEntries);
  });

  it('throws with status 503 when service returns unavailable payload', async () => {
    getJsonMock.mockResolvedValue({
      error: 'Workbench not running',
      _status: 503,
    });
    await expect(loadReadLaterEntries()).rejects.toMatchObject({
      status: 503,
    });
  });
});

describe('markEntryRead', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    window.__TAURI__ = { core: { invoke: invokeMock } };
  });

  afterEach(() => {
    delete window.__TAURI__;
  });

  it('invokes mark_read_later with id and read true', async () => {
    invokeMock.mockResolvedValue({
      entry: { ...sampleEntries[0], read: true },
      _status: 200,
    });
    await markEntryRead('abc123');
    expect(invokeMock).toHaveBeenCalledWith('mark_read_later', {
      id: 'abc123',
      read: true,
    });
  });
});

describe('bindFocusRefresh', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('calls refresh on window focus', () => {
    const refresh = vi.fn().mockResolvedValue(undefined);
    const dispose = bindFocusRefresh(refresh);
    window.dispatchEvent(new Event('focus'));
    expect(refresh).toHaveBeenCalledTimes(1);
    dispose();
  });

  it('calls refresh when document becomes visible', () => {
    const refresh = vi.fn().mockResolvedValue(undefined);
    const dispose = bindFocusRefresh(refresh);
    Object.defineProperty(document, 'visibilityState', {
      configurable: true,
      value: 'visible',
    });
    document.dispatchEvent(new Event('visibilitychange'));
    expect(refresh).toHaveBeenCalledTimes(1);
    dispose();
  });

  it('does not call refresh when document becomes hidden', () => {
    const refresh = vi.fn().mockResolvedValue(undefined);
    const dispose = bindFocusRefresh(refresh);
    Object.defineProperty(document, 'visibilityState', {
      configurable: true,
      value: 'hidden',
    });
    document.dispatchEvent(new Event('visibilitychange'));
    expect(refresh).not.toHaveBeenCalled();
    dispose();
  });

  it('dispose removes listeners so later events do not refresh', () => {
    const refresh = vi.fn().mockResolvedValue(undefined);
    const dispose = bindFocusRefresh(refresh);
    dispose();
    window.dispatchEvent(new Event('focus'));
    expect(refresh).not.toHaveBeenCalled();
  });
});

describe('mountReadLaterList', () => {
  let container;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    getJsonMock.mockReset();
    invokeMock.mockReset();
    window.__TAURI__ = { core: { invoke: invokeMock } };
    Object.defineProperty(document, 'visibilityState', {
      configurable: true,
      value: 'visible',
    });
  });

  afterEach(() => {
    container.remove();
    delete window.__TAURI__;
  });

  it('renders title, url, and saved_at after mount', async () => {
    getJsonMock.mockResolvedValue([sampleEntries[0]]);
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-item')).not.toBeNull();
    });
    const item = container.querySelector('.read-later-item');
    expect(item.querySelector('.read-later-title')?.textContent).toBe(
      'Unread Article',
    );
    expect(item.querySelector('.read-later-url')?.textContent).toBe(
      'https://example.com/unread',
    );
    expect(item.querySelector('.read-later-saved-at')?.textContent).toBe(
      '2026-07-01T10:00:00Z',
    );
    dispose();
  });

  it('distinguishes unread and read entry styles', async () => {
    getJsonMock.mockResolvedValue(sampleEntries);
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelectorAll('.read-later-item')).toHaveLength(2);
    });
    expect(
      container.querySelector('[data-entry-id="abc123"]')?.classList.contains(
        'read-later-item--unread',
      ),
    ).toBe(true);
    expect(
      container.querySelector('[data-entry-id="def456"]')?.classList.contains(
        'read-later-item--read',
      ),
    ).toBe(true);
    dispose();
  });

  it('shows empty state for empty list without error', async () => {
    getJsonMock.mockResolvedValue([]);
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-empty')).not.toBeNull();
    });
    expect(container.querySelector('.read-later-unavailable')).toBeNull();
    dispose();
  });

  it('refreshes list on window focus', async () => {
    getJsonMock.mockResolvedValue([sampleEntries[0]]);
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-item')).not.toBeNull();
    });
    getJsonMock.mockResolvedValue([
      { ...sampleEntries[0], title: 'Updated After Focus' },
    ]);
    window.dispatchEvent(new Event('focus'));
    await vi.waitFor(() => {
      expect(getJsonMock).toHaveBeenCalledTimes(2);
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-title')?.textContent).toBe(
        'Updated After Focus',
      );
    });
    dispose();
  });

  it('refreshes list when document visibility becomes visible', async () => {
    getJsonMock.mockResolvedValue([sampleEntries[0]]);
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-item')).not.toBeNull();
    });
    getJsonMock.mockResolvedValue([
      { ...sampleEntries[0], title: 'Updated After Visibility' },
    ]);
    document.dispatchEvent(new Event('visibilitychange'));
    await vi.waitFor(() => {
      expect(getJsonMock).toHaveBeenCalledTimes(2);
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-title')?.textContent).toBe(
        'Updated After Visibility',
      );
    });
    dispose();
  });

  it('keeps prior snapshot visible while focus refresh is in flight', async () => {
    let resolveRefresh;
    getJsonMock
      .mockResolvedValueOnce([sampleEntries[0]])
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveRefresh = resolve;
          }),
      );
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-item')).not.toBeNull();
    });
    window.dispatchEvent(new Event('focus'));
    expect(container.querySelector('.read-later-item')).not.toBeNull();
    expect(container.querySelector('.read-later-empty')).toBeNull();
    resolveRefresh([{ ...sampleEntries[0], title: 'After Slow Refresh' }]);
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-title')?.textContent).toBe(
        'After Slow Refresh',
      );
    });
    dispose();
  });

  it('shows error-empty on first mount GET failure (connection error)', async () => {
    getJsonMock.mockRejectedValue(new Error('Failed to fetch'));
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-empty')).not.toBeNull();
      expect(container.querySelector('.read-later-unavailable')).not.toBeNull();
    });
    expect(container.querySelector('.read-later-item')).toBeNull();
    expect(container.textContent).not.toContain('请先启动 Workbench');
    dispose();
  });

  it('shows error-empty on first mount handler failure', async () => {
    getJsonMock.mockRejectedValue(Object.assign(new Error('HTTP 500'), { status: 500 }));
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-empty')).not.toBeNull();
      expect(container.querySelector('.read-later-unavailable')).not.toBeNull();
    });
    expect(container.querySelector('.read-later-item')).toBeNull();
    dispose();
  });

  it('does not use stale 503-specific unavailable copy', async () => {
    getJsonMock.mockResolvedValue({
      error: 'Workbench not running',
      _status: 503,
    });
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-unavailable')).not.toBeNull();
    });
    expect(container.textContent).not.toContain('请先启动 Workbench');
    dispose();
  });

  it('retains snapshot with unavailable banner on focus refresh handler error', async () => {
    getJsonMock.mockResolvedValueOnce([sampleEntries[0]]);
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-item')).not.toBeNull();
    });
    getJsonMock.mockRejectedValueOnce(
      Object.assign(new Error('HTTP 500'), { status: 500 }),
    );
    window.dispatchEvent(new Event('focus'));
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-item')).not.toBeNull();
      expect(container.querySelector('.read-later-unavailable')).not.toBeNull();
    });
    expect(container.querySelector('.read-later-empty')).toBeNull();
    dispose();
  });

  it('retains snapshot with unavailable banner on focus refresh connection error', async () => {
    getJsonMock.mockResolvedValueOnce(sampleEntries);
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelectorAll('.read-later-item')).toHaveLength(2);
    });
    getJsonMock.mockRejectedValueOnce(new Error('Failed to fetch'));
    window.dispatchEvent(new Event('focus'));
    await vi.waitFor(() => {
      expect(container.querySelectorAll('.read-later-item')).toHaveLength(2);
      expect(container.querySelector('.read-later-unavailable')).not.toBeNull();
    });
    dispose();
  });

  it('mark read button invokes mark_read_later and updates UI', async () => {
    getJsonMock.mockResolvedValue([sampleEntries[0]]);
    invokeMock.mockResolvedValue({
      entry: { ...sampleEntries[0], read: true },
    });
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-mark-read')).not.toBeNull();
    });
    container.querySelector('.read-later-mark-read').click();
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('mark_read_later', {
        id: 'abc123',
        read: true,
      });
    });
    await vi.waitFor(() => {
      expect(
        container
          .querySelector('[data-entry-id="abc123"]')
          ?.classList.contains('read-later-item--read'),
      ).toBe(true);
    });
    dispose();
  });

  it('shows action error and re-enables button when mark_read_later fails', async () => {
    getJsonMock.mockResolvedValue([sampleEntries[0]]);
    invokeMock.mockRejectedValue(new Error('IPC failed'));
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-mark-read')).not.toBeNull();
    });
    const btn = container.querySelector('.read-later-mark-read');
    btn.click();
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-action-error')).not.toBeNull();
    });
    expect(btn.disabled).toBe(false);
    expect(
      container
        .querySelector('[data-entry-id="abc123"]')
        ?.classList.contains('read-later-item--unread'),
    ).toBe(true);
    dispose();
  });

  it('dispose clears container and stops updates', async () => {
    getJsonMock.mockImplementation(
      () =>
        new Promise((resolve) => {
          setTimeout(() => resolve(sampleEntries), 50);
        }),
    );
    const { dispose } = mountReadLaterList(container);
    dispose();
    expect(container.innerHTML).toBe('');
    await new Promise((r) => setTimeout(r, 60));
    expect(container.innerHTML).toBe('');
  });
});

describe('renderUnavailableState', () => {
  let container;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
  });

  afterEach(() => {
    container.remove();
  });

  it('renders error-empty with empty state and unavailable banner', () => {
    renderUnavailableState(container, { mode: 'empty', message: UNAVAILABLE_MSG });
    expect(container.querySelector('.read-later-empty')).not.toBeNull();
    expect(container.querySelector('.read-later-unavailable')?.textContent).toBe(
      UNAVAILABLE_MSG,
    );
    expect(container.querySelector('.read-later-item')).toBeNull();
  });

  it('renders error-retained with list snapshot and unavailable banner', () => {
    renderUnavailableState(container, {
      mode: 'retained',
      message: UNAVAILABLE_MSG,
      entries: sampleEntries,
    });
    expect(container.querySelector('.read-later-unavailable')?.textContent).toBe(
      UNAVAILABLE_MSG,
    );
    expect(container.querySelectorAll('.read-later-item')).toHaveLength(2);
    expect(container.querySelector('.read-later-empty')).toBeNull();
  });
});

describe('TAC-9 main window unavailable UI', () => {
  let container;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    getJsonMock.mockReset();
    invokeMock.mockReset();
    window.__TAURI__ = { core: { invoke: invokeMock } };
    Object.defineProperty(document, 'visibilityState', {
      configurable: true,
      value: 'visible',
    });
  });

  afterEach(() => {
    container.remove();
    delete window.__TAURI__;
  });

  function expectUnavailableMessage(root) {
    expect(root.querySelector('.read-later-unavailable')?.textContent).toBe(
      UNAVAILABLE_MSG,
    );
  }

  it('error-empty: local_http connection failure shows empty state and unavailable banner', async () => {
    getJsonMock.mockRejectedValue(new Error('Failed to fetch'));
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-empty')).not.toBeNull();
      expectUnavailableMessage(container);
    });
    expect(container.querySelector('.read-later-item')).toBeNull();
    dispose();
  });

  it('error-empty: GET HTTP 404 shows empty state and unavailable banner', async () => {
    getJsonMock.mockRejectedValue(Object.assign(new Error('HTTP 404'), { status: 404 }));
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-empty')).not.toBeNull();
      expectUnavailableMessage(container);
    });
    expect(container.querySelector('.read-later-item')).toBeNull();
    dispose();
  });

  it('error-empty: GET HTTP 400 shows empty state and unavailable banner', async () => {
    getJsonMock.mockRejectedValue(Object.assign(new Error('HTTP 400'), { status: 400 }));
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-empty')).not.toBeNull();
      expectUnavailableMessage(container);
    });
    expect(container.querySelector('.read-later-item')).toBeNull();
    dispose();
  });

  it('error-empty: GET HTTP 500 shows empty state and unavailable banner', async () => {
    getJsonMock.mockRejectedValue(Object.assign(new Error('HTTP 500'), { status: 500 }));
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-empty')).not.toBeNull();
      expectUnavailableMessage(container);
    });
    expect(container.querySelector('.read-later-item')).toBeNull();
    dispose();
  });

  it('error-retained: GET failure after snapshot retains list and shows unavailable banner', async () => {
    getJsonMock.mockResolvedValueOnce([sampleEntries[0]]);
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-item')).not.toBeNull();
    });
    getJsonMock.mockRejectedValueOnce(new Error('Failed to fetch'));
    window.dispatchEvent(new Event('focus'));
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-item')).not.toBeNull();
      expectUnavailableMessage(container);
    });
    expect(container.querySelector('.read-later-empty')).toBeNull();
    dispose();
  });

  it('does not silently succeed when mark_read_later invoke fails', async () => {
    getJsonMock.mockResolvedValue([sampleEntries[0]]);
    invokeMock.mockRejectedValue(new Error('IPC failed'));
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-mark-read')).not.toBeNull();
    });
    container.querySelector('.read-later-mark-read').click();
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-action-error')).not.toBeNull();
    });
    expect(
      container
        .querySelector('[data-entry-id="abc123"]')
        ?.classList.contains('read-later-item--unread'),
    ).toBe(true);
    expect(container.querySelector('.read-later-unavailable')).toBeNull();
    dispose();
  });

  it('does not fake read success when mark_read_later returns service error payload', async () => {
    getJsonMock.mockResolvedValue([sampleEntries[0]]);
    invokeMock.mockResolvedValue({
      error: 'mark_read_later failed',
      _status: 500,
    });
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-mark-read')).not.toBeNull();
    });
    container.querySelector('.read-later-mark-read').click();
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-action-error')).not.toBeNull();
    });
    expect(
      container
        .querySelector('[data-entry-id="abc123"]')
        ?.classList.contains('read-later-item--unread'),
    ).toBe(true);
    dispose();
  });
});
