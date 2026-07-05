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
  loadReadLaterEntries,
  markEntryRead,
  mountReadLaterList,
} from '../frontend/js/components/read-later-list.js';

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
    let unmountReadLaterList;

    beforeEach(() => {
      seedReadLaterRouteDom();
      unmountReadLaterList = null;
      getJsonMock.mockResolvedValue([sampleEntries[0]]);
    });

    afterEach(() => {
      unmountReadLaterList?.();
      unmountReadLaterList = null;
    });

    function invokeMountReadLaterRoute() {
      const factory = createMountReadLaterRouteFromMain();
      expect(factory).not.toBeNull();
      const mountReadLaterRoute = factory(
        null,
        null,
        unmountReadLaterList,
        document.getElementById('feed-view'),
        mountReadLaterList,
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
      return mountReadLaterRoute();
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

describe('mountReadLaterList', () => {
  let container;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    getJsonMock.mockReset();
    invokeMock.mockReset();
    window.__TAURI__ = { core: { invoke: invokeMock } };
  });

  afterEach(() => {
    container.remove();
    delete window.__TAURI__;
  });

  it('renders title, url, and saved_at after mount', async () => {
    getJsonMock.mockResolvedValue([sampleEntries[0]]);
    const { unmount } = mountReadLaterList(container);
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
    unmount();
  });

  it('distinguishes unread and read entry styles', async () => {
    getJsonMock.mockResolvedValue(sampleEntries);
    const { unmount } = mountReadLaterList(container);
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
    unmount();
  });

  it('shows empty state for empty list without error', async () => {
    getJsonMock.mockResolvedValue([]);
    const { unmount } = mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.read-later-empty')).not.toBeNull();
    });
    expect(container.querySelector('.read-later-error')).toBeNull();
    unmount();
  });

  it('shows 请先启动 Workbench on 503', async () => {
    getJsonMock.mockResolvedValue({
      error: 'Workbench not running',
      _status: 503,
    });
    mountReadLaterList(container);
    await vi.waitFor(() => {
      expect(container.textContent).toContain('请先启动 Workbench');
    });
  });

  it('mark read button invokes mark_read_later and updates UI', async () => {
    getJsonMock.mockResolvedValue([sampleEntries[0]]);
    invokeMock.mockResolvedValue({
      entry: { ...sampleEntries[0], read: true },
    });
    const { unmount } = mountReadLaterList(container);
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
    unmount();
  });

  it('unmount clears container and stops updates', async () => {
    getJsonMock.mockImplementation(
      () =>
        new Promise((resolve) => {
          setTimeout(() => resolve(sampleEntries), 50);
        }),
    );
    const { unmount } = mountReadLaterList(container);
    unmount();
    expect(container.innerHTML).toBe('');
    await new Promise((r) => setTimeout(r, 60));
    expect(container.innerHTML).toBe('');
  });
});
