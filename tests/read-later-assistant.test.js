// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const openUrlMock = vi.fn();
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
  loadAssistantEntries,
  mountReadLaterAssistant,
  openExternalUrl,
  selectTop3Unread,
} from '../frontend/js/read-later-assistant.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const assistantHtml = readFileSync(
  join(fixtureRoot, 'frontend/read-later-assistant.html'),
  'utf8',
);

const sampleEntries = [
  {
    id: 'a1',
    url: 'https://example.com/oldest-unread',
    title: 'Oldest Unread',
    saved_at: '2026-07-01T10:00:00Z',
    read: false,
  },
  {
    id: 'r1',
    url: 'https://example.com/read',
    title: 'Read Item',
    saved_at: '2026-07-05T10:00:00Z',
    read: true,
  },
  {
    id: 'm1',
    url: 'https://example.com/mid-unread',
    title: 'Mid Unread',
    saved_at: '2026-07-03T10:00:00Z',
    read: false,
  },
  {
    id: 'n1',
    url: 'https://example.com/newest-unread',
    title: 'Newest Unread',
    saved_at: '2026-07-06T10:00:00Z',
    read: false,
  },
  {
    id: 'x1',
    url: 'https://example.com/extra-unread-4',
    title: 'Extra Unread 4',
    saved_at: '2026-07-04T10:00:00Z',
    read: false,
  },
  {
    id: 'x2',
    url: 'https://example.com/extra-unread-5',
    title: 'Extra Unread 5',
    saved_at: '2026-07-02T10:00:00Z',
    read: false,
  },
];

describe('read-later-assistant source wiring', () => {
  it('read-later-assistant.html includes assistant root shell', () => {
    expect(assistantHtml).toMatch(/id="read-later-assistant-root"/);
  });

  it('read-later-assistant.html loads read-later-assistant.js module', () => {
    expect(assistantHtml).toMatch(/read-later-assistant\.js/);
  });
});

describe('loadAssistantEntries', () => {
  beforeEach(() => {
    getJsonMock.mockReset();
  });

  it('GET /api/read-later via apiClient and returns entries array', async () => {
    getJsonMock.mockResolvedValue(sampleEntries);
    const entries = await loadAssistantEntries();
    expect(getJsonMock).toHaveBeenCalledWith('/api/read-later');
    expect(entries).toEqual(sampleEntries);
  });

  it('throws with status when service returns unavailable payload', async () => {
    getJsonMock.mockResolvedValue({
      error: 'Workbench not running',
      _status: 503,
    });
    await expect(loadAssistantEntries()).rejects.toMatchObject({
      status: 503,
    });
  });
});

describe('selectTop3Unread', () => {
  it('filters unread, sorts saved_at desc, and slices to 3', () => {
    const top3 = selectTop3Unread(sampleEntries);
    expect(top3).toHaveLength(3);
    expect(top3.map((e) => e.id)).toEqual(['n1', 'x1', 'm1']);
  });

  it('returns fewer than 3 when unread count is below 3', () => {
    const entries = [sampleEntries[0], sampleEntries[1], sampleEntries[2]];
    const top3 = selectTop3Unread(entries);
    expect(top3).toHaveLength(2);
    expect(top3.map((e) => e.id)).toEqual(['m1', 'a1']);
  });

  it('returns empty array when no unread entries', () => {
    expect(selectTop3Unread([sampleEntries[1]])).toEqual([]);
    expect(selectTop3Unread([])).toEqual([]);
  });
});

describe('openExternalUrl', () => {
  beforeEach(() => {
    openUrlMock.mockReset();
    openUrlMock.mockResolvedValue(undefined);
    window.__TAURI__ = { opener: { openUrl: openUrlMock } };
  });

  afterEach(() => {
    delete window.__TAURI__;
  });

  it('opens url in system browser via Tauri opener', async () => {
    await openExternalUrl('https://example.com/article');
    expect(openUrlMock).toHaveBeenCalledWith('https://example.com/article');
  });
});

describe('mountReadLaterAssistant', () => {
  let root;

  beforeEach(() => {
    root = document.createElement('div');
    root.id = 'read-later-assistant-root';
    document.body.appendChild(root);
    getJsonMock.mockReset();
    openUrlMock.mockReset();
    openUrlMock.mockResolvedValue(undefined);
    window.__TAURI__ = { opener: { openUrl: openUrlMock } };
  });

  afterEach(() => {
    root.remove();
    delete window.__TAURI__;
  });

  it('loads full GET on mount and displays Top3 unread', async () => {
    getJsonMock.mockResolvedValue(sampleEntries);
    const { dispose } = mountReadLaterAssistant(root);
    await vi.waitFor(() => {
      expect(root.querySelector('.read-later-assistant-panel')).not.toBeNull();
    });
    expect(getJsonMock).toHaveBeenCalledWith('/api/read-later');
    expect(root.querySelectorAll('.read-later-assistant-picker-item')).toHaveLength(3);
    expect(root.querySelector('.read-later-assistant-current-title')?.textContent).toBe(
      'Newest Unread',
    );
    dispose();
  });

  it('shows empty state when no unread entries', async () => {
    getJsonMock.mockResolvedValue([sampleEntries[1]]);
    const { dispose } = mountReadLaterAssistant(root);
    await vi.waitFor(() => {
      expect(root.querySelector('.read-later-assistant-empty')).not.toBeNull();
    });
    expect(root.querySelector('.read-later-assistant-picker-item')).toBeNull();
    expect(root.querySelector('.read-later-assistant-unavailable')).toBeNull();
    dispose();
  });

  it('shows unavailable empty state on mount GET failure without fake unread', async () => {
    getJsonMock.mockRejectedValue(new Error('Failed to fetch'));
    const { dispose } = mountReadLaterAssistant(root);
    await vi.waitFor(() => {
      expect(root.querySelector('.read-later-assistant-empty')).not.toBeNull();
      expect(root.querySelector('.read-later-assistant-unavailable')).not.toBeNull();
    });
    expect(root.querySelector('.read-later-assistant-picker-item')).toBeNull();
    expect(root.querySelector('.read-later-assistant-current-title')).toBeNull();
    dispose();
  });

  it('shows unavailable empty state on handler 500 without fake unread', async () => {
    getJsonMock.mockRejectedValue(Object.assign(new Error('HTTP 500'), { status: 500 }));
    const { dispose } = mountReadLaterAssistant(root);
    await vi.waitFor(() => {
      expect(root.querySelector('.read-later-assistant-unavailable')).not.toBeNull();
    });
    expect(root.querySelector('.read-later-assistant-picker-item')).toBeNull();
    dispose();
  });

  it('updates current display when selecting another Top3 item', async () => {
    getJsonMock.mockResolvedValue(sampleEntries);
    const { dispose } = mountReadLaterAssistant(root);
    await vi.waitFor(() => {
      expect(root.querySelectorAll('.read-later-assistant-picker-item')).toHaveLength(3);
    });
    const second = root.querySelector(
      '.read-later-assistant-picker-item[data-entry-id="x1"]',
    );
    expect(second).not.toBeNull();
    second.click();
    expect(root.querySelector('.read-later-assistant-current-title')?.textContent).toBe(
      'Extra Unread 4',
    );
    dispose();
  });

  it('updates current display via next and previous controls', async () => {
    getJsonMock.mockResolvedValue(sampleEntries);
    const { dispose } = mountReadLaterAssistant(root);
    await vi.waitFor(() => {
      expect(root.querySelector('.read-later-assistant-next')).not.toBeNull();
    });
    expect(root.querySelector('.read-later-assistant-current-title')?.textContent).toBe(
      'Newest Unread',
    );
    root.querySelector('.read-later-assistant-next').click();
    expect(root.querySelector('.read-later-assistant-current-title')?.textContent).toBe(
      'Extra Unread 4',
    );
    root.querySelector('.read-later-assistant-prev').click();
    expect(root.querySelector('.read-later-assistant-current-title')?.textContent).toBe(
      'Newest Unread',
    );
    dispose();
  });

  it('opens current entry url in system browser when open link is clicked', async () => {
    getJsonMock.mockResolvedValue(sampleEntries);
    const { dispose } = mountReadLaterAssistant(root);
    await vi.waitFor(() => {
      expect(root.querySelector('.read-later-assistant-open-link')).not.toBeNull();
    });
    root.querySelector('.read-later-assistant-open-link').click();
    await vi.waitFor(() => {
      expect(openUrlMock).toHaveBeenCalledWith('https://example.com/newest-unread');
    });
    dispose();
  });

  it('does not expose mark_read controls in assistant v1', async () => {
    getJsonMock.mockResolvedValue(sampleEntries);
    const { dispose } = mountReadLaterAssistant(root);
    await vi.waitFor(() => {
      expect(root.querySelector('.read-later-assistant-panel')).not.toBeNull();
    });
    expect(root.querySelector('.read-later-mark-read')).toBeNull();
    dispose();
  });

  it('dispose clears root and stops updates', async () => {
    getJsonMock.mockImplementation(
      () =>
        new Promise((resolve) => {
          setTimeout(() => resolve(sampleEntries), 50);
        }),
    );
    const { dispose } = mountReadLaterAssistant(root);
    dispose();
    expect(root.innerHTML).toBe('');
    await new Promise((r) => setTimeout(r, 60));
    expect(root.innerHTML).toBe('');
  });
});
