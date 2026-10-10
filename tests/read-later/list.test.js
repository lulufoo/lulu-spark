// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { readShellHtml } from '../helpers/read-frontend-js.js';

const invokeMock = vi.fn();
const getJsonMock = vi.fn();
const openReadLaterInChat = vi.hoisted(() => vi.fn().mockResolvedValue());

vi.mock('../../frontend/src/read-later/commands/open-in-chat.ts', () => ({
  openReadLaterInChat,
  readLaterReferenceDraft: vi.fn(),
}));

vi.mock('../../frontend/src/host/apiClient.ts', async (importOriginal) => {
  const actual = await importOriginal();
  return {
    ...actual,
    resolveReadDriver: vi.fn(() => ({ getJson: getJsonMock })),
    createApiClient: (driver) => ({
      getJson: driver?.getJson ?? getJsonMock,
    }),
  };
});

import { parseHash } from '../../frontend/src/router/index.ts';
import { readMainSource } from '../helpers/read-frontend-js.js';
import {
  bindFocusRefresh,
  deleteReadLaterEntry,
  loadReadLaterEntries,
  markEntryRead,
  mountReadLaterList,
  renderUnavailableState,
} from '../../frontend/src/read-later/ui/list.tsx';

const UNAVAILABLE_MSG = 'List temporarily unavailable. Please try again later';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const mainJs = readMainSource();
const indexHtml = readShellHtml();

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
    <div id="knowledge-doc-view" style="display:block;"></div>
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
    'unmountKnowledgeDocList',
    'unmountReadLaterList',
    'feedView',
    'mountReadLaterList',
    'hideHomeView',
    'hideKnowledgeDocView',
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
    it('Read Later list lives in the dialog, not a page slot', () => {
      expect(indexHtml).toMatch(/id="read-later-dialog"/);
      expect(indexHtml).not.toMatch(/id="read-later-view"/);
    });

    it('main.js defines mountReadLaterRoute', () => {
      expect(mainJs).toMatch(/function mountReadLaterRoute/);
    });

    it('main.js registers read-later via wrapRouteMount in setRouteHandlers', () => {
      expect(mainJs).toMatch(/setRouteHandlers\s*\(/);
      expect(mainJs).toMatch(
        /['"]read-later['"]:\s*wrapRouteMount\s*\(\s*['"]read-later['"]\s*,\s*mountReadLaterRoute/,
      );
    });

    it('mountReadLaterRoute opens read-later dialog on home', () => {
      const body = extractFunctionBody(mainJs, 'mountReadLaterRoute');
      expect(body).toMatch(/mountHomeRoute\s*\(/);
      expect(body).toMatch(/openReadLaterDialog\s*\(/);
    });

    it('index.html includes read-later dialog shell', () => {
      expect(indexHtml).toMatch(/id="read-later-dialog"/);
      expect(indexHtml).toMatch(/id="read-later-dialog-body"/);
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
    it('mountReadLaterRoute is wired to home + dialog in main.js', () => {
      expect(mainJs).toMatch(/openReadLaterDialog/);
      expect(mainJs).toMatch(/mountHomeRoute\(\)/);
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
      error: 'Spark not running',
      _status: 503,
    });
    await expect(loadReadLaterEntries()).rejects.toMatchObject({
      status: 503,
    });
  });
});

describe('deleteReadLaterEntry', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    window.__TAURI__ = { core: { invoke: invokeMock } };
  });

  afterEach(() => {
    delete window.__TAURI__;
  });

  it('invokes delete_read_later with id', async () => {
    invokeMock.mockResolvedValue({ entry: sampleEntries[0] });
    await deleteReadLaterEntry('abc123');
    expect(invokeMock).toHaveBeenCalledWith('delete_read_later', { id: 'abc123' });
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
    const openUrlMock = vi.fn().mockResolvedValue(undefined);
    window.__TAURI__ = {
      core: { invoke: invokeMock },
      opener: { openUrl: openUrlMock },
    };
    Object.defineProperty(document, 'visibilityState', {
      configurable: true,
      value: 'visible',
    });
  });

  afterEach(() => {
    container.remove();
    delete window.__TAURI__;
  });

  it('renders unread links sorted by saved_at after mount', async () => {
    getJsonMock.mockResolvedValue(sampleEntries);
    const { dispose } = mountReadLaterList(container, { showTabs: false, initialFilter: 'unread' });
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-item')).not.toBeNull();
    });
    expect(container.querySelectorAll('.read-later-item')).toHaveLength(1);
    const link = container.querySelector('.read-later-link');
    expect(link?.querySelector('.read-later-link-title')?.textContent).toBe(
      'Unread Article',
    );
    expect(link?.getAttribute('href')).toBe('https://example.com/unread');
    dispose();
  });

  it('hides read entries in default unread filter', async () => {
    getJsonMock.mockResolvedValue(sampleEntries);
    const { dispose } = mountReadLaterList(container, { showTabs: false, initialFilter: 'unread' });
    await vi.waitFor(() => {
      expect(container.querySelectorAll('.read-later-item')).toHaveLength(1);
    });
    expect(container.querySelector('[data-entry-id="def456"]')).toBeNull();
    dispose();
  });

  it('shows all entries when filter is all', async () => {
    getJsonMock.mockResolvedValue(sampleEntries);
    const { dispose } = mountReadLaterList(container, { showTabs: true, initialFilter: 'all' });
    await vi.waitFor(() => {
      expect(container.querySelectorAll('.read-later-item')).toHaveLength(2);
    });
    dispose();
  });

  it('switches between all and unread tabs', async () => {
    getJsonMock.mockResolvedValue(sampleEntries);
    const { dispose } = mountReadLaterList(container, { showTabs: true, initialFilter: 'unread' });
    await vi.waitFor(() => {
      expect(container.querySelectorAll('.read-later-item')).toHaveLength(1);
    });
    container.querySelector('[data-filter="all"]').click();
    expect(container.querySelectorAll('.read-later-item')).toHaveLength(2);
    container.querySelector('[data-filter="unread"]').click();
    expect(container.querySelectorAll('.read-later-item')).toHaveLength(1);
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
      expect(container.querySelector('.read-later-link-title')?.textContent).toBe(
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
      expect(container.querySelector('.read-later-link-title')?.textContent).toBe(
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
      expect(container.querySelector('.read-later-link-title')?.textContent).toBe(
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
    expect(container.textContent).not.toContain('请先启动 Spark');
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

  it('keeps the unavailable banner when switching tabs after first-load failure', async () => {
    getJsonMock.mockRejectedValue(new Error('Failed to fetch'));
    const { dispose } = mountReadLaterList(container, {
      showTabs: true,
      initialFilter: 'unread',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-unavailable')).not.toBeNull();
    });
    container.querySelector('[data-filter="all"]').click();
    expect(container.querySelector('.read-later-unavailable')).not.toBeNull();
    expect(container.querySelector('.read-later-empty')).not.toBeNull();
    expect(container.querySelector('.read-later-item')).toBeNull();
    dispose();
  });

  it('keeps the unavailable banner when switching tabs after a failed refresh', async () => {
    getJsonMock.mockResolvedValueOnce(sampleEntries);
    const { dispose } = mountReadLaterList(container, {
      showTabs: true,
      initialFilter: 'unread',
    });
    await vi.waitFor(() => {
      expect(container.querySelectorAll('.read-later-item')).toHaveLength(1);
    });
    getJsonMock.mockRejectedValueOnce(new Error('Failed to fetch'));
    window.dispatchEvent(new Event('focus'));
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-unavailable')).not.toBeNull();
    });
    container.querySelector('[data-filter="all"]').click();
    expect(container.querySelector('.read-later-unavailable')).not.toBeNull();
    expect(container.querySelectorAll('.read-later-item')).toHaveLength(2);
    dispose();
  });

  it('does not use stale 503-specific unavailable copy', async () => {
    getJsonMock.mockResolvedValue({
      error: 'Spark not running',
      _status: 503,
    });
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-unavailable')).not.toBeNull();
    });
    expect(container.textContent).not.toContain('请先启动 Spark');
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

  it('retains unread snapshot with unavailable banner on focus refresh connection error', async () => {
    getJsonMock.mockResolvedValueOnce([sampleEntries[0]]);
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelectorAll('.read-later-item')).toHaveLength(1);
    });
    getJsonMock.mockRejectedValueOnce(new Error('Failed to fetch'));
    window.dispatchEvent(new Event('focus'));
    await vi.waitFor(() => {
      expect(container.querySelectorAll('.read-later-item')).toHaveLength(1);
      expect(container.querySelector('.read-later-unavailable')).not.toBeNull();
    });
    dispose();
  });

  it('link click opens url, marks read, and keeps item in all tab', async () => {
    getJsonMock.mockResolvedValue([sampleEntries[0]]);
    invokeMock.mockResolvedValue({
      entry: { ...sampleEntries[0], read: true },
    });
    const { dispose } = mountReadLaterList(container, { showTabs: true, initialFilter: 'all' });
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-link')).not.toBeNull();
    });
    container.querySelector('.read-later-link').click();
    await vi.waitFor(() => {
      expect(window.__TAURI__.opener.openUrl).toHaveBeenCalledWith(
        'https://example.com/unread',
      );
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
    container.querySelector('[data-filter="unread"]').click();
    expect(container.querySelector('[data-entry-id="abc123"]')).toBeNull();
    dispose();
  });

  it('open-in-chat button prefills chat and does not open the url', async () => {
    getJsonMock.mockResolvedValue([sampleEntries[0]]);
    const { dispose } = mountReadLaterList(container, { showTabs: true, initialFilter: 'all' });
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-open-in-chat')).not.toBeNull();
    });
    expect(container.querySelector('.read-later-open-in-chat')?.textContent?.trim()).toBe('');
    expect(container.querySelector('.read-later-open-in-chat [data-viewer-icon="chat"]')).not.toBeNull();
    container.querySelector('.read-later-open-in-chat').click();
    await vi.waitFor(() => {
      expect(openReadLaterInChat).toHaveBeenCalledWith(
        expect.objectContaining({ id: 'abc123', url: 'https://example.com/unread' }),
        expect.any(HTMLButtonElement),
      );
    });
    expect(window.__TAURI__.opener.openUrl).not.toHaveBeenCalled();
    dispose();
  });

  it('delete button removes entry from list', async () => {
    getJsonMock.mockResolvedValue([sampleEntries[0]]);
    invokeMock.mockResolvedValue({ entry: sampleEntries[0] });
    const { dispose } = mountReadLaterList(container, { showTabs: true, initialFilter: 'all' });
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-delete')).not.toBeNull();
    });
    container.querySelector('.read-later-delete').click();
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('delete_read_later', { id: 'abc123' });
      expect(container.querySelector('[data-entry-id="abc123"]')).toBeNull();
    });
    dispose();
  });

  it('shows action error when mark_read_later fails after link open', async () => {
    getJsonMock.mockResolvedValue([sampleEntries[0]]);
    invokeMock.mockRejectedValue(new Error('IPC failed'));
    const { dispose } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-link')).not.toBeNull();
    });
    container.querySelector('.read-later-link').click();
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-action-error')).not.toBeNull();
    });
    expect(container.querySelector('[data-entry-id="abc123"]')).not.toBeNull();
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
      filter: 'all',
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
    const openUrlMock = vi.fn().mockResolvedValue(undefined);
    window.__TAURI__ = {
      core: { invoke: invokeMock },
      opener: { openUrl: openUrlMock },
    };
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
      expect(container.querySelector('.read-later-link')).not.toBeNull();
    });
    container.querySelector('.read-later-link').click();
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-action-error')).not.toBeNull();
    });
    expect(container.querySelector('[data-entry-id="abc123"]')).not.toBeNull();
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
      expect(container.querySelector('.read-later-link')).not.toBeNull();
    });
    container.querySelector('.read-later-link').click();
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-action-error')).not.toBeNull();
    });
    expect(container.querySelector('[data-entry-id="abc123"]')).not.toBeNull();
    dispose();
  });
});
