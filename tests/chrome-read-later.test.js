import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import { WORKBENCH_BASE } from '../extensions/chrome-read-later/lib/config.js';
import { save } from '../extensions/chrome-read-later/lib/readLaterApi.js';
import { badgeFeedbackForResult } from '../extensions/chrome-read-later/lib/feedback.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const manifestPath = resolve(
  __dirname,
  '../extensions/chrome-read-later/manifest.json'
);

describe('chrome-read-later config', () => {
  it('WORKBENCH_BASE points at local Workbench HTTP', () => {
    expect(WORKBENCH_BASE).toBe('http://127.0.0.1:8765');
  });
});

describe('readLaterApi.save', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('POSTs url and title to /api/read-later and returns entry on 201', async () => {
    const entry = {
      id: 'abc123',
      url: 'https://example.com',
      title: 'Example',
      saved_at: '2026-07-02T00:00:00Z',
      read: false,
    };
    fetch.mockResolvedValue({
      status: 201,
      json: async () => entry,
    });

    const result = await save({
      url: 'https://example.com',
      title: 'Example',
    });

    expect(fetch).toHaveBeenCalledWith(
      `${WORKBENCH_BASE}/api/read-later`,
      expect.objectContaining({
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          url: 'https://example.com',
          title: 'Example',
        }),
      })
    );
    expect(result).toEqual({ ok: true, status: 201, entry });
  });

  it('returns ok:false with status 503 when Workbench is unavailable', async () => {
    fetch.mockResolvedValue({
      status: 503,
      json: async () => ({ error: 'Workbench not running' }),
    });

    const result = await save({
      url: 'https://example.com',
      title: 'Example',
    });

    expect(result).toEqual({
      ok: false,
      status: 503,
      error: 'Workbench not running',
    });
  });

  it('returns ok:false with status 0 on network failure', async () => {
    fetch.mockRejectedValue(new TypeError('Failed to fetch'));

    const result = await save({
      url: 'https://example.com',
      title: 'Example',
    });

    expect(result.ok).toBe(false);
    expect(result.status).toBe(0);
    expect(result.error).toMatch(/fetch/i);
  });
});

describe('badgeFeedbackForResult', () => {
  it('shows success badge when save succeeds', () => {
    expect(
      badgeFeedbackForResult({ ok: true, status: 201 })
    ).toEqual({
      badgeText: 'OK',
      title: '已保存到待读',
    });
  });

  it('shows Workbench offline message on 503', () => {
    expect(
      badgeFeedbackForResult({ ok: false, status: 503 })
    ).toEqual({
      badgeText: '!',
      title: '请先启动 Workbench',
    });
  });

  it('shows network error message on status 0', () => {
    expect(
      badgeFeedbackForResult({
        ok: false,
        status: 0,
        error: 'Failed to fetch',
      })
    ).toEqual({
      badgeText: '!',
      title: '网络错误，请检查 Workbench',
    });
  });
});

describe('manifest.json', () => {
  it('is MV3 with activeTab and localhost host_permissions only', () => {
    const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
    expect(manifest.manifest_version).toBe(3);
    expect(manifest.permissions).toEqual(['activeTab']);
    expect(manifest.host_permissions).toEqual(['http://127.0.0.1:8765/*']);
    expect(manifest.background?.service_worker).toBe('background.js');
    expect(manifest.action?.default_icon).toBeTruthy();
    expect(manifest.content_scripts).toBeUndefined();
    expect(manifest.action?.default_popup).toBeUndefined();
  });
});
