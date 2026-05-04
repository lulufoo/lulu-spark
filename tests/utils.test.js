import { escHtml, formatDate, timeFromTs, slugToTitle, nowTs, importanceBadgeHtml, filenameFromPath, topicFromPath } from '../js/utils.js'
import { test, expect } from 'vitest'

// escHtml
test('escHtml 转义 < > &', () => {
  expect(escHtml('<script>alert(1)</script>')).toBe('&lt;script&gt;alert(1)&lt;/script&gt;')
})
test('escHtml 转义 &', () => {
  expect(escHtml('a & b')).toBe('a &amp; b')
})
test('escHtml 空字符串', () => {
  expect(escHtml('')).toBe('')
})

// slugToTitle
test('slugToTitle 连字符转首字母大写', () => {
  expect(slugToTitle('hello-world')).toBe('Hello World')
})
test('slugToTitle 去掉时间戳前缀', () => {
  expect(slugToTitle('202605041230-my-note')).toBe('My Note')
})
test('slugToTitle 单词', () => {
  expect(slugToTitle('vitest')).toBe('Vitest')
})

// filenameFromPath
test('filenameFromPath 去除路径和 .md', () => {
  expect(filenameFromPath('ai/learning/my-note.md')).toBe('my-note')
})
test('filenameFromPath 根目录文件', () => {
  expect(filenameFromPath('note.md')).toBe('note')
})

// topicFromPath
test('topicFromPath 返回目录部分', () => {
  expect(topicFromPath('ai/learning/my-note.md')).toBe('ai/learning')
})
test('topicFromPath 单层路径', () => {
  expect(topicFromPath('ai/note.md')).toBe('ai')
})

// formatDate
test('formatDate 返回正确的 day / label', () => {
  const r = formatDate('20260504')
  expect(r.day).toBe('4日')
  expect(r.label).toBe('5月4日')
  expect(r.year).toBe('2026')
  expect(r.month).toBe('2026/05')
})

// timeFromTs
test('timeFromTs 从12位时间戳提取 HH:MM', () => {
  expect(timeFromTs('202605041430')).toBe('14:30')
})
test('timeFromTs 零点', () => {
  expect(timeFromTs('202605040000')).toBe('00:00')
})

// nowTs
test('nowTs 返回12位纯数字字符串', () => {
  const ts = nowTs()
  expect(ts).toMatch(/^\d{12}$/)
})

// importanceBadgeHtml
test('importanceBadgeHtml high 含正确 class', () => {
  const html = importanceBadgeHtml('high')
  expect(html).toContain('badge-importance-high')
  expect(html).toContain('↑ 高')
})
test('importanceBadgeHtml medium', () => {
  const html = importanceBadgeHtml('medium')
  expect(html).toContain('badge-importance-medium')
  expect(html).toContain('→ 中')
})
test('importanceBadgeHtml low', () => {
  const html = importanceBadgeHtml('low')
  expect(html).toContain('badge-importance-low')
  expect(html).toContain('↓ 低')
})
test('importanceBadgeHtml undefined 返回 unset button', () => {
  const html = importanceBadgeHtml(undefined)
  expect(html).toContain('badge-importance-unset')
  expect(html).toContain('☆')
})
