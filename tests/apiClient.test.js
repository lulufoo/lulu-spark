import { test, expect, vi, beforeEach } from 'vitest';

const invokeMock = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args) => invokeMock(...args),
}));

import {
  createApiClient,
  createFetchDriver,
  resolveReadDriver,
} from '../js/apiClient.js';

const DEFAULT_DEV_BASE = 'http://127.0.0.1:8765';

function mockFetchJson(body, ok = true, status = 200) {
  globalThis.fetch = vi.fn().mockResolvedValue({
    ok,
    status,
    json: () => Promise.resolve(body),
  });
}

beforeEach(() => {
  vi.restoreAllMocks();
  invokeMock.mockReset();
});

test('fetchDriver getJson 对 /api/topics 发起 GET 并返回解析后 JSON', async () => {
  const payload = { topics: ['a'] };
  mockFetchJson(payload);
  const driver = createFetchDriver();
  const result = await driver.getJson('/api/topics');

  expect(fetch).toHaveBeenCalledOnce();
  const [url, init] = fetch.mock.calls[0];
  expect(url).toBe(`${DEFAULT_DEV_BASE}/api/topics`);
  expect(init).toEqual({ method: 'GET' });
  expect(result).toEqual(payload);
});

test('createApiClient 绑定 fetchDriver 时 getJson 走 HTTP', async () => {
  mockFetchJson({ ok: true });
  const client = createApiClient(createFetchDriver());
  await client.getJson('/api/config');
  expect(fetch).toHaveBeenCalledOnce();
  expect(invokeMock).not.toHaveBeenCalled();
});

test('query 含需 encodeURIComponent 的字符时 URL 拼接正确且不二次编码', async () => {
  mockFetchJson({});
  const driver = createFetchDriver();
  const encoded = encodeURIComponent('ai/my note.md');
  const path = `/api/annotation?path=${encoded}`;
  await driver.getJson(path);

  const url = fetch.mock.calls[0][0];
  expect(url).toBe(`${DEFAULT_DEV_BASE}${path}`);
  expect(url).toContain(encoded);
  expect(url).not.toMatch(/%2520/);
});

test('HTTP 非 2xx 时 getJson 抛出 Error（含 HTTP 状态）', async () => {
  mockFetchJson({ error: 'nope' }, false, 503);
  const driver = createFetchDriver();
  await expect(driver.getJson('/api/config')).rejects.toThrow('HTTP 503');
});

test('resolveReadDriver 在 fetch 模式下不调用 invoke', async () => {
  mockFetchJson({});
  const driver = resolveReadDriver('fetch');
  await driver.getJson('/api/topics');
  expect(fetch).toHaveBeenCalled();
  expect(invokeMock).not.toHaveBeenCalled();
});

test('resolveReadDriver 默认（未设置 VITE_READ_API）为 fetch', () => {
  expect(resolveReadDriver()).toBe('fetch');
});
