import { describe, it, expect, beforeEach } from 'vitest'
import { getCorpusGithub, setCorpusGithub } from '../js/constants.js'

const DEFAULT = 'https://github.com/lulufoo/lulu-workbench-knowledge/blob/main'

describe('setCorpusGithub / getCorpusGithub', () => {
  beforeEach(() => {
    setCorpusGithub(DEFAULT)
  })

  it('默认基址为 knowledge 仓库', () => {
    expect(getCorpusGithub()).toBe(DEFAULT)
  })

  it('运行时 setCorpusGithub 可覆盖', () => {
    setCorpusGithub('https://example.com/blob/main/')
    expect(getCorpusGithub()).toBe('https://example.com/blob/main')
  })

  it('空值回退默认基址', () => {
    setCorpusGithub('')
    expect(getCorpusGithub()).toBe(DEFAULT)
  })
})
