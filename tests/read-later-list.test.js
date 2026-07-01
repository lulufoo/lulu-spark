// @vitest-environment jsdom
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

import {
  loadReadLaterEntries,
  markEntryRead,
  mountReadLaterList,
} from '../frontend/js/components/read-later-list.js';

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
