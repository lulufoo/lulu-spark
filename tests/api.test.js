import { readFileSync } from 'node:fs'
import { test, expect, vi, beforeEach } from 'vitest'
import { DEFAULT_DEV_BASE } from '../frontend/js/apiClient.js'
import {
  assertReadPayload,
  assertWritePayload,
  fetchIndex, fetchDiffStatus, fetchAnnotationsSummary, fetchAnnotation,
  fetchKbDiffStatus,
  fetchConfig, fetchFileContent, fetchLinkTitle,
  setConfig,
  saveFile, commitFiles, pullProject, revertFile,
  updateComments, updateLinks, setImportance, setDone,
  deleteEntry, ghMove,
  fetchTopics, moveToProject,
} from '../frontend/js/api.js'

const API_READ_PREFIX = `${DEFAULT_DEV_BASE}/api`

function mockFetch(body, ok = true, status = 200) {
  globalThis.fetch = vi.fn().mockResolvedValue({
    ok,
    status,
    json: () => Promise.resolve(body),
    text: () => Promise.resolve(typeof body === 'string' ? body : JSON.stringify(body))
  })
}

// 模拟服务器返回 HTML 页面（如 Python BaseHTTPServer 的默认 404 响应）
function mockFetchHtml(status = 404) {
  globalThis.fetch = vi.fn().mockResolvedValue({
    ok: status >= 200 && status < 300,
    status,
    json: () => Promise.reject(new SyntaxError(
      `Unexpected token '<', "<!DOCTYPE "... is not valid JSON`
    )),
    text: () => Promise.resolve('<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 4.01//EN">\n<html><body>File not found</body></html>')
  })
}

beforeEach(() => { vi.restoreAllMocks() })

test('api.js 无裸 fetch（读路径均经 readGet / writePost）', () => {
  const src = readFileSync(new URL('../frontend/js/api.js', import.meta.url), 'utf8')
  expect([...src.matchAll(/await fetch\(/g)]).toHaveLength(0)
})

// ── GET endpoints ──────────────────────────────────────────────────────────

test('fetchIndex 调用 /api/corpus-index 并返回 JSON', async () => {
  mockFetch({ entries: [] })
  const result = await fetchIndex()
  expect(fetch).toHaveBeenCalledOnce()
  expect(fetch.mock.calls[0][0]).toMatch(
    new RegExp(`^${API_READ_PREFIX}/corpus-index`)
  )
  expect(result).toEqual({ entries: [] })
})

test('fetchIndex 在非 2xx 时抛出错误', async () => {
  mockFetch({}, false, 500)
  await expect(fetchIndex()).rejects.toThrow('HTTP 500')
})

test('assertReadPayload 在 Tauri 风格 error 对象上抛出', () => {
  expect(() =>
    assertReadPayload({ error: 'No such file: /tmp/index.json', _status: 404 }),
  ).toThrow('No such file')
})

test('assertWritePayload 在 Tauri 风格 error 对象上抛出', () => {
  expect(() =>
    assertWritePayload({ error: 'Invalid common_path', _status: 400 }),
  ).toThrow('Invalid common_path')
})

test('fetchIndex 在 JSON body 含 error 时抛出（Tauri 路径）', async () => {
  mockFetch({ error: 'No such file: /tmp/workbench-x/index.json', _status: 404 })
  await expect(fetchIndex()).rejects.toThrow('No such file')
})

test('fetchDiffStatus 调用 /api/status', async () => {
  mockFetch({ modified: ['raw/a.md'], conflicted: [] })
  const result = await fetchDiffStatus()
  expect(fetch.mock.calls[0][0]).toMatch(new RegExp(`^${API_READ_PREFIX}/status`))
  expect(result.modified).toEqual(['raw/a.md'])
})

test('fetchDiffStatus 非 2xx 时抛出错误', async () => {
  mockFetch({}, false, 503)
  await expect(fetchDiffStatus()).rejects.toThrow('HTTP 503')
})

test('fetchAnnotationsSummary 调用 /api/annotations', async () => {
  mockFetch({ 'ai/note.md': { done: true } })
  const result = await fetchAnnotationsSummary()
  expect(fetch.mock.calls[0][0]).toMatch(new RegExp(`^${API_READ_PREFIX}/annotations`))
  expect(result['ai/note.md'].done).toBe(true)
})

test('fetchAnnotation 对 path 做 encodeURIComponent', async () => {
  mockFetch({})
  await fetchAnnotation('ai/my note.md')
  expect(fetch.mock.calls[0][0]).toContain(encodeURIComponent('ai/my note.md'))
})

test('fetchConfig 调用 /api/config', async () => {
  mockFetch({ workbench_knowledge_root: '/tmp' })
  const result = await fetchConfig()
  expect(fetch.mock.calls[0][0]).toBe(`${API_READ_PREFIX}/config`)
  expect(result.workbench_knowledge_root).toBe('/tmp')
})

test('setConfig 发送 POST 到 /api/config', async () => {
  mockFetch({ has_github_token: true })
  await setConfig({ github_token: 'ghp_test' })
  expect(fetch.mock.calls[0][0]).toBe(`${DEFAULT_DEV_BASE}/api/config`)
  expect(fetch.mock.calls[0][1].method).toBe('POST')
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.github_token).toBe('ghp_test')
})

test('fetchFileContent 调用 /api/corpus-file 并返回 content', async () => {
  mockFetch({ content: '# Hello' })
  const text = await fetchFileContent('raw', 'ai/note.md')
  expect(fetch.mock.calls[0][0]).toMatch(
    new RegExp(
      `^${API_READ_PREFIX}/corpus-file\\?layer=raw&path=${encodeURIComponent('ai/note.md')}`
    )
  )
  expect(text).toBe('# Hello')
})

test('fetchFileContent 非 2xx 时抛出错误', async () => {
  mockFetch({}, false, 404)
  await expect(fetchFileContent('raw', 'missing.md')).rejects.toThrow('HTTP 404')
})

test('fetchLinkTitle 对 url 做 encodeURIComponent', async () => {
  mockFetch({ title: 'My Page' })
  const result = await fetchLinkTitle('https://example.com/a b')
  expect(fetch.mock.calls[0][0]).toMatch(
    new RegExp(`${API_READ_PREFIX}/fetch-title\\?url=${encodeURIComponent('https://example.com/a b')}`)
  )
  expect(result.title).toBe('My Page')
})

test('fetchKbDiffStatus 调用 /api/kb/diff-status 并返回 JSON', async () => {
  mockFetch({ repos: [{ full_name: 'o/r', has_changes: true }] })
  const result = await fetchKbDiffStatus()
  expect(fetch.mock.calls[0][0]).toMatch(
    new RegExp(`^${API_READ_PREFIX}/kb/diff-status`)
  )
  expect(result).toEqual({ repos: [{ full_name: 'o/r', has_changes: true }] })
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
  expect(fetch.mock.calls[0][0]).toBe(`${DEFAULT_DEV_BASE}/api/pull`)
  expect(fetch.mock.calls[0][1].method).toBe('POST')
})

test('revertFile 发送 path 和 type 到 /api/corpus-revert', async () => {
  mockFetch({ ok: true })
  const result = await revertFile('raw/foo/bar.md', 'modified')
  expect(fetch.mock.calls[0][0]).toBe(`${DEFAULT_DEV_BASE}/api/corpus-revert`)
  expect(fetch.mock.calls[0][1].method).toBe('POST')
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.path).toBe('raw/foo/bar.md')
  expect(body.type).toBe('modified')
  expect(result).toEqual({ ok: true })
})

test('revertFile 空 path 和 type 执行全量 revert', async () => {
  mockFetch({ ok: true })
  await revertFile('', '')
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.path).toBe('')
  expect(body.type).toBe('')
})

test('revertFile undefined 参数保底为空字符串', async () => {
  mockFetch({ ok: true })
  await revertFile(undefined, undefined)
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.path).toBe('')
  expect(body.type).toBe('')
})

test('revertFile 后端返回 error 时抛出', async () => {
  mockFetch({ error: 'revert failed', _status: 500 })
  await expect(revertFile('raw/foo/bar.md', 'modified')).rejects.toThrow('revert failed')
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

// ── fetchTopics ────────────────────────────────────────────────────────────

test('fetchTopics 调用 /api/topics?_=<timestamp> 并返回数据', async () => {
  mockFetch({ topics: [{ dir: 'ai', description: 'AI notes' }] })
  const result = await fetchTopics()
  expect(fetch.mock.calls[0][0]).toMatch(new RegExp(`^${API_READ_PREFIX}/topics\\?_=\\d+$`))
  expect(result.topics).toHaveLength(1)
  expect(result.topics[0].dir).toBe('ai')
})

test('fetchTopics topics.json 不存在时抛出服务器返回的错误信息', async () => {
  mockFetch({ error: 'repo-list.json not found; run ⊙ 全量同步 in the app' }, false, 404)
  await expect(fetchTopics()).rejects.toThrow('repo-list.json not found')
})

// 回归测试：复现 BUG —— 服务器返回 HTML 404 时 res.json() 在 res.ok 检查前抛出
// SyntaxError，导致用户看到 "Unexpected token '<'..." 而非可读错误。
// 修复后此测试应通过；修复前会 FAIL。
test('fetchTopics 服务器返回 HTML 时抛出可读错误而非 JSON 解析异常', async () => {
  mockFetchHtml(404)
  const err = await fetchTopics().catch(e => e)
  expect(err).toBeInstanceOf(Error)
  expect(err.message).not.toMatch(/Unexpected token/)
  expect(err.message).toMatch(/HTTP 404/)
})

// ── moveToProject ──────────────────────────────────────────────────────────

test('moveToProject 发送正确 POST body 并返回响应', async () => {
  const id = 'a'.repeat(32)
  mockFetch({ ok: true, new_common_path: 'ai/note.md' })
  const result = await moveToProject(id, 'ai')
  expect(fetch.mock.calls[0][0]).toBe(`${DEFAULT_DEV_BASE}/api/move-project`)
  expect(fetch.mock.calls[0][1].method).toBe('POST')
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.id).toBe(id)
  expect(body.new_project).toBe('ai')
  expect(result.ok).toBe(true)
})

// 回归测试：moveToProject 也存在同样的 res.json() 前置问题，
// 服务器返回 HTML 时应抛出可读错误而非 JSON 解析异常。
test('moveToProject 服务器返回 HTML 时不抛出 JSON 解析异常', async () => {
  mockFetch({ error: 'not found' }, false, 404)
  const err = await moveToProject('a'.repeat(32), 'ai').catch(e => e)
  expect(err).toBeInstanceOf(Error)
  expect(err.message).toBe('not found')
})
