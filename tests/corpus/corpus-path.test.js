import { describe, it, expect } from 'vitest'
import { getActivePath } from '../../frontend/src/corpus/corpus-path.ts'

describe('getActivePath', () => {
  const entry = {
    common_path: 'proj/doc.md',
    translations: { zh: 'proj/zh/doc.md' },
  }

  it('raw 层 zh 时返回 translations.zh', () => {
    expect(getActivePath(entry, 'zh', 'raw')).toBe('proj/zh/doc.md')
  })

  it('digest 层 zh 时仍返回 common_path', () => {
    expect(getActivePath(entry, 'zh', 'digest')).toBe('proj/doc.md')
  })

  it('raw 层 en 时返回 common_path', () => {
    expect(getActivePath(entry, null, 'raw')).toBe('proj/doc.md')
  })

  it('无 zh 翻译时返回 common_path', () => {
    expect(getActivePath({ common_path: 'a/b.md' }, 'zh', 'raw')).toBe('a/b.md')
  })
})
