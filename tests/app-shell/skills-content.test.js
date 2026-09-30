import { existsSync, readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test, expect } from 'vitest'
import { readFrontendJs, readShellHtml } from '../helpers/read-frontend-js.js';
import { workbenchSkillsContent } from '../../frontend/src/app-shell/state/skills-content.ts'

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..')

const PUBLIC_CMDS = [
  'dialogue-summary',
  'dialogue-archive',
  'theme-line',
  'theme-fetch',
  'theme-transcribe',
  'note-task',
]

test('workbench catalog exposes exactly the public child skills', () => {
  expect(workbenchSkillsContent.title).toBe('✦ Lulu Workbench Skills')
  expect(workbenchSkillsContent.groups).toHaveLength(PUBLIC_CMDS.length)

  const cmds = workbenchSkillsContent.groups.flatMap((group) =>
    group.items.map((item) => item.cmd)
  )

  expect(cmds).toEqual(PUBLIC_CMDS)
  expect(new Set(cmds).size).toBe(PUBLIC_CMDS.length)
})

test('each public child skill has one clickable item with required display fields', () => {
  const cjk = /[\u4e00-\u9fff]/
  workbenchSkillsContent.groups.forEach((group) => {
    expect(group.name).toEqual(expect.any(String))
    expect(group.name).not.toBe('')
    expect(group.name).not.toMatch(cjk)
    expect(group.items).toHaveLength(1)

    const item = group.items[0]
    expect(group.url).toContain(
      `lulu-workbench-skills/tree/main/${item.cmd}`
    )
    expect(item.name).toBe(group.name)
    expect(item.name).not.toMatch(cjk)
    expect(item.desc).toEqual(expect.any(String))
    expect(item.desc).not.toBe('')
    expect(item.desc).not.toMatch(cjk)
  })
})

test('skills dialog renders one English table row per skill and no group links', () => {
  const src = readFrontendJs('frontend/src/app-shell/ui/skills-dialog.tsx')
  expect(src).toMatch(/class(?:Name)?="skill-table"/)
  expect(src).toMatch(/class(?:Name)?="skill-name"/)
  expect(src).toMatch(/class(?:Name)?="skill-cmd"/)
  expect(src).not.toContain('skill-group-link')
  expect(src).not.toContain('skill-group-title')
})

test('workbench catalog does not expose legacy or internal skills', () => {
  const allText = JSON.stringify(workbenchSkillsContent)

  expect(allText).not.toContain('theme-summary')
  expect(allText).not.toContain('plan-task')
  expect(allText).not.toMatch(/dtd_/)
  expect(allText).not.toContain('digest')
})

test('SKILL dropdown keeps only Lulu Workbench Skills', () => {
  const html = readShellHtml()
  expect(html).toContain('id="btn-skill-workbench"')
  expect(html).toContain('✦ Lulu Workbench Skills')
  expect(html).not.toContain('id="btn-skill-lulu"')
  expect(html).not.toContain('id="btn-skill-software-dev"')
  expect(html).not.toContain('Lulu Learning Skills')
  expect(html).not.toContain('Lulu Dev Skills')
  expect(
    existsSync(join(repoRoot, 'frontend/src/skills-software-dev-content.js')),
  ).toBe(false)
})

test('skills dialog is a React component without import-time menu listener', () => {
  const src = [
    readFrontendJs('frontend/src/app-shell/ui/skills-dialog.tsx'),
    readFrontendJs('frontend/src/app-shell/commands/skills-dialog.ts'),
    readFrontendJs('frontend/src/app-shell/state/skills.ts'),
  ].join('\n')
  expect(src).toMatch(/export function SkillsDialog/)
  expect(src).toMatch(/export function _openSkillsDialog/)
  expect(src).toMatch(/export function _closeSkillsDialog/)
  expect(src).toMatch(/createModuleStore/)
  expect(src).toMatch(/useSyncExternalStore/)
  expect(src).toMatch(/id="skills-dialog"/)
  expect(src).toMatch(/id="skills-dialog-title"/)
  expect(src).toMatch(/id="skills-dialog-body"/)
  expect(src).toMatch(/id="btn-skills-dialog-close"[\s\S]*?onClick/)
  expect(src).toMatch(/e\.target === e\.currentTarget/)
  expect(src).not.toMatch(/getElementById\(\s*['"]btn-skill-workbench['"]\s*\)/)
  expect(src).not.toMatch(/btn-skill-workbench['"]\s*\)!\s*\.addEventListener/)
})
