import { test, expect, vi, beforeEach } from 'vitest';

const invokeMock = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args) => invokeMock(...args),
  Channel: function Channel(onmessage) {
    this.onmessage = onmessage;
  },
}));

import {
  createApiClient,
  createChannel,
  createFetchDriver,
  createTauriDriver,
  resolveReadDriver,
} from '../../frontend/js/host/apiClient.js';

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

test('tauriDriver getJson 通过 invoke 调用映射命令', async () => {
  invokeMock.mockResolvedValue({ total: 1, modified: ['raw/a.md'] });
  const driver = createTauriDriver();
  const result = await driver.getJson('/api/status?_=' + Date.now());

  expect(invokeMock).toHaveBeenCalledOnce();
  expect(invokeMock).toHaveBeenCalledWith('get_status', {});
  expect(result.total).toBe(1);
});

test('tauriDriver 在未配置映射时抛出可读错误', async () => {
  const driver = createTauriDriver();
  await expect(driver.getJson('/api/not-exists')).rejects.toThrow(
    'No Tauri invoke mapping for /api/not-exists'
  );
  expect(invokeMock).not.toHaveBeenCalled();
});

test('tauriDriver postJson 通过 invoke 调用写映射命令', async () => {
  invokeMock.mockResolvedValue({ ok: true });
  const driver = createTauriDriver();
  const res = await driver.postJson('/api/set-done', {
    common_path: 'ai/x.md',
    done: true,
  });
  expect(invokeMock).toHaveBeenCalledWith('set_done', {
    commonPath: 'ai/x.md',
    done: true,
  });
  expect(res.ok).toBe(true);
  expect(await res.json()).toEqual({ ok: true });
});

test('tauriDriver postJson /api/update-links 映射 commonPath + links', async () => {
  invokeMock.mockResolvedValue({ ok: true });
  const driver = createTauriDriver();
  const links = [{ url: 'https://github.com/foo/bar' }];
  const res = await driver.postJson('/api/update-links', {
    common_path: 'inbox/new-note.md',
    links,
  });
  expect(invokeMock).toHaveBeenCalledWith('update_links', {
    commonPath: 'inbox/new-note.md',
    links,
  });
  expect(invokeMock.mock.calls[0][1]).not.toHaveProperty('common_path');
  expect(res.ok).toBe(true);
});

test('tauriDriver postJson invoke 返回 Invalid common_path 时透传 400', async () => {
  invokeMock.mockResolvedValue({
    error: 'Invalid common_path',
    _status: 400,
  });
  const driver = createTauriDriver();
  const res = await driver.postJson('/api/update-links', {
    common_path: '../evil.md',
    links: [{ url: 'https://example.com' }],
  });
  expect(res.ok).toBe(false);
  expect(res.status).toBe(400);
  expect((await res.json()).error).toBe('Invalid common_path');
});

test('tauriDriver postJson /api/config 映射到 set_config', async () => {
  invokeMock.mockResolvedValue({ has_github_token: true });
  const driver = createTauriDriver();
  const res = await driver.postJson('/api/config', { github_token: 'ghp_xxx' });
  expect(invokeMock).toHaveBeenCalledWith('set_config', {
    payload: { github_token: 'ghp_xxx' },
  });
  expect(await res.json()).toEqual({ has_github_token: true });
});

test('tauriDriver postJson 未知 path 抛出可读错误', async () => {
  const driver = createTauriDriver();
  await expect(driver.postJson('/api/not-mapped', {})).rejects.toThrow(
    'No Tauri invoke mapping for POST /api/not-mapped'
  );
  expect(invokeMock).not.toHaveBeenCalled();
});

test('tauriDriver postJson invoke 返回 error+_status 404 时 ok 为 false', async () => {
  invokeMock.mockResolvedValue({ error: 'file not found', _status: 404 });
  const driver = createTauriDriver();
  const res = await driver.postJson('/api/set-done', {
    common_path: 'missing.md',
    done: true,
  });
  expect(res.ok).toBe(false);
  expect(res.status).toBe(404);
  const data = await res.json();
  expect(data.error).toBe('file not found');
});

test('createChannel 用动态 import 的 Channel 构造请求维回调管', async () => {
  const onmessage = vi.fn();
  const channel = await createChannel(onmessage);
  expect(channel.onmessage).toBe(onmessage);
});
