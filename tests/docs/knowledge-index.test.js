import { describe, expect, it } from 'vitest'
import { normalizeKnowledgeIndex } from '../../frontend/src/knowledge/state/index.ts'

describe('normalizeKnowledgeIndex', () => {
  it('从 entries 克隆并附加 _id', () => {
    const frozen = Object.freeze({ common_path: 'a/b.md', date: '2026-01-01' })
    const out = normalizeKnowledgeIndex({ entries: { e1: frozen } })
    expect(out.e1._id).toBe('e1')
    expect(out.e1.common_path).toBe('a/b.md')
    out.e1.done = true
    expect(out.e1.done).toBe(true)
  })

  it('顶层 error 抛出可读消息', () => {
    expect(() =>
      normalizeKnowledgeIndex({ error: 'No such file: /tmp/spark-x/index.json', _status: 404 }),
    ).toThrow('No such file')
  })

  it('误把 error 对象当 entries 时不会写 readonly', () => {
    expect(() =>
      normalizeKnowledgeIndex({ error: 'fail', _status: 500 }),
    ).toThrow('fail')
  })

  it('跳过非对象条目', () => {
    const out = normalizeKnowledgeIndex({ entries: { ok: { date: 'x' }, bad: 'str' } })
    expect(Object.keys(out)).toEqual(['ok'])
  })
})
