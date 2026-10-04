import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import { SPARK_BASE } from '../../extensions/chrome-spark-extension/read-later/lib/config.js';
import { save } from '../../extensions/chrome-spark-extension/read-later/lib/readLaterApi.js';
import { badgeFeedbackForResult } from '../../extensions/chrome-spark-extension/read-later/lib/feedback.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const extRoot = resolve(__dirname, '../../extensions/chrome-spark-extension');
const manifestPath = resolve(extRoot, 'manifest.json');

describe('chrome-spark-extension config', () => {
  it('SPARK_BASE points at the local Spark Gateway', () => {
    expect(SPARK_BASE).toBe('https://localhost:7654');
  });
});

describe('readLaterApi.save', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('POSTs url and title to the Gateway read-later route and returns entry on 201', async () => {
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
      `${SPARK_BASE}/read-later`,
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

  it('returns ok:false with status 0 on network failure when Spark is unavailable', async () => {
    fetch.mockRejectedValue(new TypeError('Failed to fetch'));

    const result = await save({
      url: 'https://example.com',
      title: 'Example',
    });

    expect(result.ok).toBe(false);
    expect(result.status).toBe(0);
    expect(result.error).toMatch(/fetch/i);
  });

  it('does not special-case HTTP 503 (handler does not return 503)', async () => {
    fetch.mockResolvedValue({
      status: 503,
      json: async () => ({}),
    });

    const result = await save({
      url: 'https://example.com',
      title: 'Example',
    });

    expect(result).toEqual({
      ok: false,
      status: 503,
      error: 'HTTP 503',
    });
  });

  it('returns the recent duplicate error and existing entry on 409', async () => {
    const existingEntry = {
      id: 'abc123',
      url: 'https://example.com',
      title: 'Example',
      saved_at: '2026-07-02T00:00:00Z',
      read: false,
    };

    fetch.mockResolvedValue({
      status: 409,
      json: async () => ({
        code: 'read_later_recent_duplicate',
        error: 'This URL was already saved within 24 hours',
        entry: existingEntry,
      }),
    });

    const result = await save({ url: 'https://example.com', title: 'Example' });

    expect(result).toEqual({
      ok: false,
      status: 409,
      code: 'read_later_recent_duplicate',
      error: 'This URL was already saved within 24 hours',
      entry: existingEntry,
    });
  });
});

describe('badgeFeedbackForResult', () => {
  it('shows success badge when save succeeds', () => {
    expect(
      badgeFeedbackForResult({ ok: true, status: 201 })
    ).toEqual({
      badgeText: 'OK',
      title: '已保存到待读',
      badgeColor: '#22c55e',
    });
  });

  it('shows Spark unavailable message on status 0 (network failure)', () => {
    expect(
      badgeFeedbackForResult({
        ok: false,
        status: 0,
        error: 'Failed to fetch',
      })
    ).toEqual({
      badgeText: '!',
      title: '请先启动 Lulu Spark',
      badgeColor: '#ef4444',
    });
  });

  it('shows the recent duplicate message for duplicate saves', () => {
    expect(
      badgeFeedbackForResult({
        ok: false,
        status: 409,
        code: 'read_later_recent_duplicate',
      })
    ).toEqual({
      badgeText: '!',
      title: '24小时内已保存',
      badgeColor: '#eab308',
    });
  });

  it.each([400, 500, 502])(
    'shows red failure badge for HTTP %s',
    (status) => {
      expect(
        badgeFeedbackForResult({
          ok: false,
          status,
          error: `HTTP ${status}`,
        })
      ).toEqual({
        badgeText: '!',
        title: `HTTP ${status}`,
        badgeColor: '#ef4444',
      });
    }
  );
});

describe('manifest.json', () => {
  it('unions read-later Gateway permission with X compose content scripts', () => {
    const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
    expect(manifest.manifest_version).toBe(3);
    expect(manifest.permissions).toEqual(['activeTab']);
    expect(manifest.host_permissions).toEqual(['https://localhost:7654/*']);
    expect(manifest.background?.service_worker).toBe('read-later/background.js');
    expect(manifest.action?.default_icon).toBeTruthy();
    expect(manifest.action?.default_popup).toBeUndefined();
    expect(manifest.content_scripts).toHaveLength(2);
    expect(manifest.content_scripts[0].matches).toEqual([
      'https://x.com/*',
      'https://twitter.com/*',
    ]);
    expect(manifest.content_scripts[1].world).toBe('MAIN');
  });
});

describe('x-zh-en control', () => {
  it('does not set visible 翻译 text on the injected control', () => {
    const src = readFileSync(resolve(extRoot, 'x-zh-en/content.js'), 'utf8');
    expect(src).not.toMatch(/textContent\s*=\s*["']翻译/);
    expect(src).toContain('aria-label');
    expect(src).toContain('createIcon');
  });
});
