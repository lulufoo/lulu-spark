import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..')
const mainPath = join(repoRoot, 'frontend/js/main.js')
const mainSource = readFileSync(mainPath, 'utf8')

/** IPC-heavy calls that must not run on background tags:reconciled. */
const FORBIDDEN_IN_HANDLER = [/\bselectDate\s*\(/, /\bloadTitles\s*\(/]

function extractAsyncArrowBody(source, constName) {
  const re = new RegExp(`const\\s+${constName}\\s*=\\s*async\\s*\\(\\)\\s*=>\\s*\\{`)
  const m = re.exec(source)
  if (!m) return null
  let depth = 1
  let i = m.index + m[0].length
  const start = i
  while (i < source.length && depth > 0) {
    const ch = source[i]
    if (ch === '{') depth += 1
    else if (ch === '}') depth -= 1
    i += 1
  }
  return depth === 0 ? source.slice(start, i - 1) : null
}

function forbiddenCallsIn(body) {
  return FORBIDDEN_IN_HANDLER.filter((re) => re.test(body)).map((re) => re.source)
}

describe('tags:reconciled handler contract (main.js)', () => {
  it('registers named onReconciled (no inline handler)', () => {
    expect(mainSource).toMatch(
      /listen\s*\(\s*['"]tags:reconciled['"]\s*,\s*onReconciled\s*\)/,
    )
  })

  it('onReconciled syncs data only — no selectDate / loadTitles', () => {
    const body = extractAsyncArrowBody(mainSource, 'onReconciled')
    expect(body, 'const onReconciled = async () => { ... } not found').not.toBeNull()
    const hits = forbiddenCallsIn(body)
    expect(
      hits,
      hits.length ? `forbidden in onReconciled: ${hits.join(', ')}` : '',
    ).toEqual([])
  })

  it('onReconciled keeps minimal reload chain', () => {
    const body = extractAsyncArrowBody(mainSource, 'onReconciled')
    expect(body).toMatch(/loadTagsRegistry/)
    expect(body).toMatch(/loadAnnotationsSummary/)
    expect(body).toMatch(/applyListFilters/)
    expect(body).toMatch(/renderSidebar/)
  })
})
