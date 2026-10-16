import { describe, it, expect } from 'vitest'
import { noteFileLang } from '../../frontend/src/knowledge/state/path.ts'

describe('noteFileLang', () => {
  const entry = {
    translations: { zh: 'proj/zh/doc.md' },
  }

  it('raw 层 zh 时返回 zh', () => {
    expect(noteFileLang(entry, 'zh', 'raw')).toBe('zh')
  })

  it('digest 层 zh 时不带 lang', () => {
    expect(noteFileLang(entry, 'zh', 'digest')).toBeUndefined()
  })

  it('raw 层 en 时不带 lang', () => {
    expect(noteFileLang(entry, 'en', 'raw')).toBeUndefined()
  })

  it('无 zh 翻译时不带 lang', () => {
    expect(noteFileLang({}, 'zh', 'raw')).toBeUndefined()
  })
})
