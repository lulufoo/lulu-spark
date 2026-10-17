import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test, expect, vi, beforeEach } from 'vitest'
import { DEFAULT_DEV_BASE } from '../../frontend/src/host/apiClient.ts'
import {
  assertReadPayload,
  assertWritePayload,
  fetchIndex, fetchAnnotationsSummary, fetchAnnotation,
  fetchConfig, fetchFileContent, fetchLinkTitle,
  setConfig,
  saveFile,
  updateComments, updateLinks, setImportance, setDone,
  deleteEntry,
  fetchWorkspaces, createWorkspace, renameWorkspace, setWorkspaceViewMode, setWorkspaceIcon, disableWorkspace, enableWorkspace, fetchDisabledWorkspaces, fetchDisabledProjects,
  fetchProjects, createProject, updateProject, disableProject, enableProject, moveToProject,
  reorderWorkspaces, reorderProjects,
  fetchHidePatterns, addHidePattern, updateHidePattern, removeHidePattern, fetchViewerState, saveViewerState,
} from '../../frontend/src/host/api.ts'

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..')
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
  const src = [
    'frontend/src/host/api.ts',
    'frontend/src/host/api/transport.ts',
    'frontend/src/host/api/spark.ts',
    'frontend/src/host/api/search.ts',
  ].map((rel) => readFileSync(join(repoRoot, rel), 'utf8')).join('\n')
  expect([...src.matchAll(/await fetch\(/g)]).toHaveLength(0)
})

// ── GET endpoints ──────────────────────────────────────────────────────────

test('fetchIndex 调用 /api/docs-index 并返回 JSON', async () => {
  mockFetch({ entries: [] })
  const result = await fetchIndex()
  expect(fetch).toHaveBeenCalledOnce()
  expect(fetch.mock.calls[0][0]).toMatch(
    new RegExp(`^${API_READ_PREFIX}/docs-index`)
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
    assertWritePayload({ error: 'Invalid id', _status: 400 }),
  ).toThrow('Invalid id')
})

test('fetchIndex 在 JSON body 含 error 时抛出（Tauri 路径）', async () => {
  mockFetch({ error: 'No such file: /tmp/spark-x/index.json', _status: 404 })
  await expect(fetchIndex()).rejects.toThrow('No such file')
})

test('fetchAnnotationsSummary 调用 /api/annotations', async () => {
  mockFetch({ 'ai/note.md': { done: true } })
  const result = await fetchAnnotationsSummary()
  expect(fetch.mock.calls[0][0]).toMatch(new RegExp(`^${API_READ_PREFIX}/annotations`))
  expect(result['ai/note.md'].done).toBe(true)
})

test('fetchAnnotation 对 path 做 encodeURIComponent', async () => {
  mockFetch({})
  await fetchAnnotation('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa')
  expect(fetch.mock.calls[0][0]).toContain('id=' + encodeURIComponent('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'))
})

test('fetchConfig 调用 /api/config', async () => {
  mockFetch({ spark_root: '/tmp' })
  const result = await fetchConfig()
  expect(fetch.mock.calls[0][0]).toBe(`${API_READ_PREFIX}/config`)
  expect(result.spark_root).toBe('/tmp')
})

test('setConfig 发送 POST 到 /api/config', async () => {
  mockFetch({ llm: { host: { key_saved: true } } })
  await setConfig({ llm_api_key: { engine: 'host', value: 'sk-test' } })
  expect(fetch.mock.calls[0][0]).toBe(`${DEFAULT_DEV_BASE}/api/config`)
  expect(fetch.mock.calls[0][1].method).toBe('POST')
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.llm_api_key).toEqual({ engine: 'host', value: 'sk-test' })
})

test('fetchFileContent 调用 /api/doc-file 并返回 content', async () => {
  mockFetch({ content: '# Hello' })
  const text = await fetchFileContent('raw', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa')
  expect(fetch.mock.calls[0][0]).toMatch(
    new RegExp(
      `^${API_READ_PREFIX}/doc-file\\?layer=raw&id=${encodeURIComponent('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa')}`
    )
  )
  expect(text).toBe('# Hello')
})

test('fetchFileContent 非 2xx 时抛出错误', async () => {
  mockFetch({}, false, 404)
  await expect(fetchFileContent('raw', 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb')).rejects.toThrow('HTTP 404')
})

test('fetchLinkTitle 对 url 做 encodeURIComponent', async () => {
  mockFetch({ title: 'My Page' })
  const result = await fetchLinkTitle('https://example.com/a b')
  expect(fetch.mock.calls[0][0]).toMatch(
    new RegExp(`${API_READ_PREFIX}/fetch-title\\?url=${encodeURIComponent('https://example.com/a b')}`)
  )
  expect(result.title).toBe('My Page')
})

// ── POST endpoints ─────────────────────────────────────────────────────────

test('saveFile 发送正确 body', async () => {
  mockFetch({ ok: true })
  await saveFile('raw', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', '# content')
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.layer).toBe('raw')
  expect(body.id).toBe('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa')
  expect(body.content).toBe('# content')
})

test('updateComments 发送正确 body', async () => {
  mockFetch({ ok: true })
  await updateComments('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 'raw', { id: '1', text: 'hi' }, '202605041200')
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.id).toBe('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa')
  expect(body.layer).toBe('raw')
  expect(body.comment).toEqual({ id: '1', text: 'hi' })
  expect(body.ts).toBe('202605041200')
})

test('updateLinks 发送正确 body', async () => {
  mockFetch({ ok: true })
  await updateLinks('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', [{ url: 'https://example.com' }])
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.id).toBe('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa')
  expect(body.links).toEqual([{ url: 'https://example.com' }])
})

test('setImportance 发送 importance: null 当值为 undefined', async () => {
  mockFetch({ ok: true })
  await setImportance('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', undefined)
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.importance).toBeNull()
})

test('setImportance 发送 importance: high', async () => {
  mockFetch({ ok: true })
  await setImportance('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 'high')
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.importance).toBe('high')
})

test('setDone 发送正确 body', async () => {
  mockFetch({ ok: true })
  await setDone('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', true)
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.id).toBe('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa')
  expect(body.done).toBe(true)
})

test('deleteEntry 发送 id', async () => {
  mockFetch({ ok: true })
  await deleteEntry('abc123')
  const body = JSON.parse(fetch.mock.calls[0][1].body)
  expect(body.id).toBe('abc123')
})

// ── moveToProject ──────────────────────────────────────────────────────────

test('moveToProject 发送正确 POST body 并返回响应', async () => {
  const id = 'a'.repeat(32)
  mockFetch({ ok: true, id })
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

test('fetchWorkspaces calls /api/workspaces', async () => {
  mockFetch({ workspaces: [{ id: 'default', name: 'Default', view_mode: 'directory', is_default: true }] })
  const result = await fetchWorkspaces()
  expect(fetch.mock.calls[0][0]).toMatch(new RegExp(`^${API_READ_PREFIX}/workspaces\\?_=\\d+$`))
  expect(result.workspaces[0].id).toBe('default')
})

test('fetchDisabledWorkspaces and fetchDisabledProjects read the disabled lists', async () => {
  mockFetch({ workspaces: [], projects: [] })
  await fetchDisabledWorkspaces()
  await fetchDisabledProjects('ws1')
  expect(fetch.mock.calls[0][0]).toMatch(new RegExp(`^${API_READ_PREFIX}/workspaces-disabled\\?_=\\d+$`))
  expect(fetch.mock.calls[1][0]).toMatch(new RegExp(`^${API_READ_PREFIX}/projects-disabled\\?workspace=ws1&_=\\d+$`))
})

test('createWorkspace posts name and view mode', async () => {
  mockFetch({ ok: true })
  await createWorkspace('Research', 'timeline')
  expect(fetch.mock.calls[0][0]).toBe(`${DEFAULT_DEV_BASE}/api/workspace-create`)
  expect(JSON.parse(fetch.mock.calls[0][1].body)).toEqual({ name: 'Research', view_mode: 'timeline' })
})

test('renameWorkspace, setWorkspaceViewMode, disableWorkspace and enableWorkspace post the workspace id', async () => {
  mockFetch({ ok: true })
  await renameWorkspace('ws1', 'Notes')
  await setWorkspaceViewMode('ws1', 'timeline')
  await disableWorkspace('ws1')
  await enableWorkspace('ws1')
  expect(fetch.mock.calls.map((call) => call[0])).toEqual([
    `${DEFAULT_DEV_BASE}/api/workspace-rename`,
    `${DEFAULT_DEV_BASE}/api/workspace-view-mode`,
    `${DEFAULT_DEV_BASE}/api/workspace-disable`,
    `${DEFAULT_DEV_BASE}/api/workspace-enable`,
  ])
  expect(fetch.mock.calls.map((call) => JSON.parse(call[1].body))).toEqual([
    { id: 'ws1', name: 'Notes' },
    { id: 'ws1', view_mode: 'timeline' },
    { id: 'ws1' },
    { id: 'ws1' },
  ])
})

test('hidden-files rules are read and written per workspace', async () => {
  mockFetch({ patterns: [{ id: 'seed00', pattern: '^\\.git$' }] })
  const listed = await fetchHidePatterns('ws1')
  expect(fetch.mock.calls[0][0]).toMatch(new RegExp(`^${API_READ_PREFIX}/hide-patterns\\?workspace=ws1&_=\\d+$`))
  expect(listed.patterns[0].id).toBe('seed00')

  mockFetch({ patterns: [] })
  await addHidePattern('ws1', '^draft$')
  await updateHidePattern('ws1', 'seed00', '^x$')
  await removeHidePattern('ws1', 'seed00')
  expect(fetch.mock.calls.map((call) => call[0])).toEqual([
    `${DEFAULT_DEV_BASE}/api/hide-pattern-add`,
    `${DEFAULT_DEV_BASE}/api/hide-pattern-update`,
    `${DEFAULT_DEV_BASE}/api/hide-pattern-remove`,
  ])
  expect(fetch.mock.calls.map((call) => JSON.parse(call[1].body))).toEqual([
    { workspace: 'ws1', pattern: '^draft$' },
    { workspace: 'ws1', id: 'seed00', pattern: '^x$' },
    { workspace: 'ws1', id: 'seed00' },
  ])
})

test('the remembered open document is read and written per workspace', async () => {
  mockFetch({ id: 'abc' })
  expect((await fetchViewerState('ws1')).id).toBe('abc')
  expect(fetch.mock.calls[0][0]).toMatch(new RegExp(`^${API_READ_PREFIX}/viewer-state\\?workspace=ws1&_=\\d+$`))
  await saveViewerState('ws1', 'abc')
  expect(fetch.mock.calls[1][0]).toBe(`${DEFAULT_DEV_BASE}/api/viewer-state`)
  expect(JSON.parse(fetch.mock.calls[1][1].body)).toEqual({ workspace: 'ws1', id: 'abc' })
})

test('fetchProjects calls /api/projects for one workspace', async () => {
  mockFetch({ projects: [{ folder: 'inbox', title: 'Inbox', description: '' }] })
  const result = await fetchProjects('ws1')
  expect(fetch.mock.calls[0][0]).toMatch(new RegExp(`^${API_READ_PREFIX}/projects\\?workspace=ws1&_=\\d+$`))
  expect(result.projects[0].folder).toBe('inbox')
})

test('project writes post workspace, project, title and description', async () => {
  mockFetch({ ok: true })
  await createProject('ws1', 'Ops', 'desc')
  await updateProject('ws1', 'ops', 'Operations', 'runbooks')
  await disableProject('ws1', 'ops')
  await enableProject('ws1', 'ops')
  expect(fetch.mock.calls.map((call) => call[0])).toEqual([
    `${DEFAULT_DEV_BASE}/api/project-create`,
    `${DEFAULT_DEV_BASE}/api/project-update`,
    `${DEFAULT_DEV_BASE}/api/project-disable`,
    `${DEFAULT_DEV_BASE}/api/project-enable`,
  ])
  expect(fetch.mock.calls.map((call) => JSON.parse(call[1].body))).toEqual([
    { workspace: 'ws1', title: 'Ops', description: 'desc' },
    { workspace: 'ws1', project: 'ops', title: 'Operations', description: 'runbooks' },
    { workspace: 'ws1', project: 'ops' },
    { workspace: 'ws1', project: 'ops' },
  ])
})

test('reorder posts the whole list of ids or folders', async () => {
  mockFetch({ ok: true })
  await reorderWorkspaces(['b', 'default', 'a'])
  await reorderProjects('ws1', ['two', 'one'])
  expect(fetch.mock.calls.map((call) => call[0])).toEqual([
    `${DEFAULT_DEV_BASE}/api/workspaces-reorder`,
    `${DEFAULT_DEV_BASE}/api/projects-reorder`,
  ])
  expect(fetch.mock.calls.map((call) => JSON.parse(call[1].body))).toEqual([
    { ids: ['b', 'default', 'a'] },
    { workspace: 'ws1', folders: ['two', 'one'] },
  ])
})

test('icons ride along on create and update only when given, and setWorkspaceIcon posts id and icon', async () => {
  mockFetch({ ok: true })
  await createWorkspace('Research', 'timeline', '03ab12')
  await createWorkspace('Plain')
  await setWorkspaceIcon('ws1', '05zz99')
  await setWorkspaceIcon('ws1', '')
  await createProject('ws1', 'Ops', 'desc', '03k7x2')
  await updateProject('ws1', 'ops', 'Operations', 'runbooks', '')
  await updateProject('ws1', 'ops', 'Operations', 'runbooks')
  expect(fetch.mock.calls.map((call) => call[0])).toEqual([
    `${DEFAULT_DEV_BASE}/api/workspace-create`,
    `${DEFAULT_DEV_BASE}/api/workspace-create`,
    `${DEFAULT_DEV_BASE}/api/workspace-icon`,
    `${DEFAULT_DEV_BASE}/api/workspace-icon`,
    `${DEFAULT_DEV_BASE}/api/project-create`,
    `${DEFAULT_DEV_BASE}/api/project-update`,
    `${DEFAULT_DEV_BASE}/api/project-update`,
  ])
  expect(fetch.mock.calls.map((call) => JSON.parse(call[1].body))).toEqual([
    { name: 'Research', view_mode: 'timeline', icon: '03ab12' },
    { name: 'Plain', view_mode: 'directory' },
    { id: 'ws1', icon: '05zz99' },
    { id: 'ws1', icon: '' },
    { workspace: 'ws1', title: 'Ops', description: 'desc', icon: '03k7x2' },
    { workspace: 'ws1', project: 'ops', title: 'Operations', description: 'runbooks', icon: '' },
    { workspace: 'ws1', project: 'ops', title: 'Operations', description: 'runbooks' },
  ])
})
