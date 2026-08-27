import { readFileSync, readdirSync, statSync } from 'node:fs'
import { dirname, join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..')
const frontendJsRoot = join(repoRoot, 'frontend/js')

/** Only apiClient may load @tauri-apps packages (dynamic import). */
const TAURI_IMPORT_ALLOWLIST = new Set(['frontend/js/host/apiClient.js'])

/** Static import or re-export from @tauri-apps (breaks bare ES modules in browser). */
const STATIC_TAURI_IMPORT =
  /^\s*import\s+(?:type\s+)?[\s\w*,{}]*\s+from\s+['"]@tauri-apps\//gm
const STATIC_TAURI_SIDE_EFFECT = /^\s*import\s+['"]@tauri-apps\//gm
const STATIC_TAURI_REEXPORT = /^\s*export\s+[\s\w*,{}]*\s+from\s+['"]@tauri-apps\//gm

/** @returns {string[]} paths relative to repo root */
function listJsFiles(dir) {
  const out = []
  for (const name of readdirSync(dir)) {
    const abs = join(dir, name)
    if (statSync(abs).isDirectory()) {
      out.push(...listJsFiles(abs))
    } else if (name.endsWith('.js')) {
      out.push(relative(repoRoot, abs))
    }
  }
  return out
}

function findStaticTauriImports(relPath) {
  const text = readFileSync(join(repoRoot, relPath), 'utf8')
  const hits = []
  for (const re of [STATIC_TAURI_IMPORT, STATIC_TAURI_SIDE_EFFECT, STATIC_TAURI_REEXPORT]) {
    for (const m of text.matchAll(re)) {
      const line = text.slice(0, m.index).split('\n').length
      hits.push({ line, snippet: m[0].trim() })
    }
  }
  return hits
}

describe('frontend Tauri import contract', () => {
  it('frontend/js has no static @tauri-apps/* imports outside apiClient.js', () => {
    const files = listJsFiles(frontendJsRoot)
    const violations = []
    for (const rel of files) {
      if (TAURI_IMPORT_ALLOWLIST.has(rel)) continue
      const hits = findStaticTauriImports(rel)
      for (const h of hits) violations.push({ file: rel, ...h })
    }
    expect(
      violations,
      violations.length
        ? violations.map((v) => `${v.file}:${v.line} ${v.snippet}`).join('\n')
        : '',
    ).toEqual([])
  })

  it('apiClient.js uses dynamic import for @tauri-apps (not static)', () => {
    const text = readFileSync(join(repoRoot, 'frontend/js/host/apiClient.js'), 'utf8')
    expect(text).toMatch(/import\s*\(\s*['"]@tauri-apps\/api\/core['"]\s*\)/)
    expect(findStaticTauriImports('frontend/js/host/apiClient.js')).toEqual([])
  })
})
