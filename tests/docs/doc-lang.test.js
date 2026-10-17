import { describe, it, expect } from 'vitest'
import { docFileLang } from '../../frontend/src/notes/state/doc-lang.ts'

describe('docFileLang', () => {
  const entry = {
    translations: { zh: 'proj/zh/doc.md' },
  }

  it('raw 层 zh 时返回 zh', () => {
    expect(docFileLang(entry, 'zh', 'raw')).toBe('zh')
  })

  it('digest 层 zh 时不带 lang', () => {
    expect(docFileLang(entry, 'zh', 'digest')).toBeUndefined()
  })

  it('raw 层 en 时不带 lang', () => {
    expect(docFileLang(entry, 'en', 'raw')).toBeUndefined()
  })

  it('无 zh 翻译时不带 lang', () => {
    expect(docFileLang({}, 'zh', 'raw')).toBeUndefined()
  })
})
