import { test, expect } from 'vitest'
import { resolveSyncInvoke, SYNC_API_INVOKE_MAP } from '../frontend/js/syncApiInvokeMap.js'

test('every sync POST path maps to a command', () => {
  const paths = [
    '/api/commit',
    '/api/pull',
    '/api/delete',
    '/api/move-project',
    '/api/gh-move',
    '/api/settle',
    '/api/draft',
    '/api/kb/commit',
    '/api/kb/revert',
    '/api/open-iterm',
  ]
  for (const path of paths) {
    expect(SYNC_API_INVOKE_MAP[path]).toBeDefined()
    const resolved = resolveSyncInvoke(path, {})
    expect(resolved?.cmd).toBe(SYNC_API_INVOKE_MAP[path].cmd)
  }
})

test('commit maps to corpus_git_commit with payload wrapper', () => {
  const r = resolveSyncInvoke('/api/commit', { message: 'x', files: ['a.md'] })
  expect(r.cmd).toBe('corpus_git_commit')
  expect(r.args.payload.message).toBe('x')
  expect(r.args.payload.files).toEqual(['a.md'])
})

test('move-project maps to move_entry_project with payload wrapper', () => {
  const r = resolveSyncInvoke('/api/move-project', { id: 'e1', new_project: 'foo' })
  expect(r.cmd).toBe('move_entry_project')
  expect(r.args.payload).toEqual({ id: 'e1', new_project: 'foo' })
})
