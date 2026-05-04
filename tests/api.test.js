import { test, expect, vi, beforeEach } from 'vitest'
import {
  fetchIndex, fetchDiffStatus, fetchAnnotationsSummary, fetchAnnotation,
  fetchConfig, fetchFileContent, fetchLinkTitle,
  saveFile, commitFiles, pullProject,
  updateComments, updateLinks, setImportance, setDone,
  deleteEntry, ghMove
} from '../js/api.js'

function mockFetch(body, ok = true, status = 200) {
  globalThis.fetch = vi.fn().mockResolvedValue({
    ok,
    status,
    json: () => Promise.resolve(body),
    text: () => Promise.resolve(typeof body === 'string' ? body : JSON.stringify(body))
  })
}

beforeEach(() => { vi.restoreAllMocks() })

// ── GET endpoints ──────────────────────────────────────────────────────────

test('fetchIndex 调用 ./index.json 并返回 JSON', async () => {
  mockFetch({ entries: [] })
  const result = await fetchIndex()
  expect(fetch).toHaveBeenCalledOnce()
  expect(fetch.mock.calls[0][0]).toMatch(/^\.\/index\.json/)
  expect(result).toEqual({ entries: [] })
})

test('fetchIndex 在非 2xx 时抛出错误', async () => {
  mockFetch({}, false, 500)
  await expect(fetchIndex()).rejects.toThrow('HTTP 500')
})

test('fetchDiffStatus 调用 /api/status', async () => {
  mockFetch({ modified: ['raw/a.md'], conflicted: [] })
  const result = await fetchDiffStatus()
  expect(fetch.mock.calls[0][0]).toMatch(/^\/api\/status/)
  expect(result.modified).toEqual(['raw/a.md'])
})

test('fetchDiffStatus 非 2xx 时返回 null', async () => {
  mockFetch({}, false, 503)
  const result = await fetchDiffStatus()
  expect(result).toBeNull()
})

test('fetchAnnotationsSummary 调用 /api/annotations', async () => {
  mockFetch({ 'ai/note.md': { done: true } })
  const result = await fetchAnnotationsSummary()
  expect(fetch.mock.calls[0][0]).toMatch(/^\/api\/annotations/)
  expect(result['ai/note.md'].done).toBe(true)
})

test('fetchAnnotation 对 path 做 encodeURIComponent', async () => {
  mockFetch({})
  await fetchAnnotation('ai/my note.md')
  expect(fetch.mock.calls[0][0]).toContain(encodeURIComponent('ai/my note.md'))
})

test('fetchConfig 调用 /api/config', async () => {
  mockFetch({ archive_root: '/tmp' })
  const result = await fetchConfig()
  expect(fetch.mock.calls[0][0]).toBe('/api/config')
  expect(result.archive_root).toBe('/tmp')
})

test('fetchFileContent 构造正确路径并返回文本', async () => {
  mockFetch('# Hello')
  const text = await fetchFileContent('raw', 'ai/note.md')
  expect(fetch.mock.calls[0][0]).toMatch(/^\.\/raw\/ai\/note\.md/)
  expect(text).toBe('# Hello')
})

test('fetchFileContent 非 2xx 时抛出错误', async () => {
  mockFetch({}, false, 404)
  await expect(fetchFileContent('raw', 'missing.md')).rejects.toThrow('HTTP 404')
})

test('fetchLinkTitle 对 url 做 encodeURIComponent', async () => {
  mockFetch({ title: 'My Page' })
  const result = await fetchLinkTitle('https://example.com/a b')
  expect(fetch.mock.calls[0][0]).toContain(encodeURIComponent('https://example.com/a b'))
  expect(result.title).toBe('My Page')
})

// ── POST endpoints ─────────────────────────────────────────────────────────

test('saveFile 发送正确 body', async () => {
  mockFetch({ ok: true })
  await saveFile('raw', 'ai/note.md', '# content')
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.layer).toBe('raw')
  expect(body.common_path).toBe('ai/note.md')
  expect(body.content).toBe('# content')
})

test('commitFiles 不传 files 时 body 无 files 字段', async () => {
  mockFetch({ ok: true })
  await commitFiles('chore: update')
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.message).toBe('chore: update')
  expect(body.files).toBeUndefined()
})

test('commitFiles 传 files 时 body 包含 files', async () => {
  mockFetch({ ok: true })
  await commitFiles('fix: note', ['raw/ai/note.md'])
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.files).toEqual(['raw/ai/note.md'])
})

test('pullProject 发送 POST 到 /api/pull', async () => {
  mockFetch({ ok: true })
  await pullProject()
  expect(fetch.mock.calls[0][0]).toBe('/api/pull')
  expect(fetch.mock.calls[0][1].method).toBe('POST')
})

test('updateComments 发送正确 body', async () => {
  mockFetch({ ok: true })
  await updateComments('ai/note.md', 'raw', { id: '1', text: 'hi' }, '202605041200')
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.common_path).toBe('ai/note.md')
  expect(body.layer).toBe('raw')
  expect(body.comment).toEqual({ id: '1', text: 'hi' })
  expect(body.ts).toBe('202605041200')
})

test('updateLinks 发送正确 body', async () => {
  mockFetch({ ok: true })
  await updateLinks('ai/note.md', [{ url: 'https://example.com' }])
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.common_path).toBe('ai/note.md')
  expect(body.links).toEqual([{ url: 'https://example.com' }])
})

test('setImportance 发送 importance: null 当值为 undefined', async () => {
  mockFetch({ ok: true })
  await setImportance('ai/note.md', undefined)
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.importance).toBeNull()
})

test('setImportance 发送 importance: high', async () => {
  mockFetch({ ok: true })
  await setImportance('ai/note.md', 'high')
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.importance).toBe('high')
})

test('setDone 发送正确 body', async () => {
  mockFetch({ ok: true })
  await setDone('ai/note.md', true)
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.common_path).toBe('ai/note.md')
  expect(body.done).toBe(true)
})

test('deleteEntry 发送 id', async () => {
  mockFetch({ ok: true })
  await deleteEntry('abc123')
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.id).toBe('abc123')
})

test('ghMove 发送 src_url 和 dst_dir_url', async () => {
  mockFetch({ ok: true })
  await ghMove('https://github.com/src', 'https://github.com/dst')
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.src_url).toBe('https://github.com/src')
  expect(body.dst_dir_url).toBe('https://github.com/dst')
})
