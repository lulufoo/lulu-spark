import { afterEach, describe, expect, it } from 'vitest'
import {
  getGithubUserUrl,
  setGithubUserUrl,
  workbenchGithubBlobBase,
} from '../../frontend/js/host/constants.js'

describe('github_user_url runtime', () => {
  afterEach(() => {
    setGithubUserUrl('')
  })

  it('默认空字符串', () => {
    expect(getGithubUserUrl()).toBe('')
  })

  it('setGithubUserUrl 去掉末尾斜杠', () => {
    setGithubUserUrl('https://github.com/lulufoo/')
    expect(getGithubUserUrl()).toBe('https://github.com/lulufoo')
  })
})

describe('workbenchGithubBlobBase', () => {
  it('从个人主页 + 本地目录名推导 blob 前缀', () => {
    expect(
      workbenchGithubBlobBase(
        'https://github.com/lulufoo',
        '/Users/me/Code/lulu-workbench-knowledge',
      ),
    ).toBe('https://github.com/lulufoo/lulu-workbench-knowledge/blob/main')
  })

  it('未配置主页时返回空', () => {
    expect(workbenchGithubBlobBase('', '/Code/lulu-workbench-knowledge')).toBe('')
  })
})
