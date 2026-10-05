import { existsSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test, expect } from 'vitest'
import { readFrontendSrcTree, readShellHtml } from '../helpers/read-frontend-js.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..')

test('product chrome does not show SKILL', () => {
  const html = readShellHtml()
  expect(html).not.toContain('btn-skills-menu')
  expect(html).not.toContain('btn-skill-spark')
  expect(html).not.toContain('✦ SKILL')
  expect(html).not.toContain('SkillsDialog')
  expect(html).not.toContain('skills-dialog')
  expect(existsSync(join(repoRoot, 'frontend/src/app-shell/ui/skills-dialog.tsx'))).toBe(false)
  expect(existsSync(join(repoRoot, 'frontend/src/app-shell/commands/skills-dialog.ts'))).toBe(false)
  expect(existsSync(join(repoRoot, 'frontend/src/app-shell/state/skills-content.ts'))).toBe(false)
  expect(existsSync(join(repoRoot, 'frontend/src/app-shell/state/skills.ts'))).toBe(false)
})

test('frontend source does not remount a skills product surface', () => {
  const src = readFrontendSrcTree()
  expect(src).not.toMatch(/_openSkillsDialog/)
  expect(src).not.toMatch(/id=["']skills-dialog["']/)
  expect(src).not.toMatch(/id=["']btn-skills-menu["']/)
})
