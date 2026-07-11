import { test, expect } from 'vitest'
import { workbenchSkillsContent } from '../frontend/js/skills-workbench-content.js'

const PUBLIC_CMDS = [
  'dialogue-summary',
  'dialogue-archive',
  'theme-line',
  'theme-fetch',
  'theme-archive',
  'plan-task',
]

test('workbench catalog exposes exactly the six public child skills', () => {
  expect(workbenchSkillsContent.title).toEqual(expect.any(String))
  expect(workbenchSkillsContent.title).not.toBe('')
  expect(workbenchSkillsContent.groups).toHaveLength(6)

  const cmds = workbenchSkillsContent.groups.flatMap((group) =>
    group.items.map((item) => item.cmd)
  )

  expect(cmds).toEqual(PUBLIC_CMDS)
  expect(new Set(cmds).size).toBe(6)
})

test('each public child skill has one clickable item with required display fields', () => {
  workbenchSkillsContent.groups.forEach((group) => {
    expect(group.name).toEqual(expect.any(String))
    expect(group.name).not.toBe('')
    expect(group.items).toHaveLength(1)

    const item = group.items[0]
    expect(group.url).toContain(
      `lulu-workbench-skills/tree/main/${item.cmd}`
    )
    expect(item.name).toEqual(expect.any(String))
    expect(item.name).not.toBe('')
    expect(item.desc).toEqual(expect.any(String))
    expect(item.desc).not.toBe('')
  })
})

test('workbench catalog does not expose legacy or internal skills', () => {
  const allText = JSON.stringify(workbenchSkillsContent)

  expect(allText).not.toContain('theme-summary')
  expect(allText).not.toMatch(/dtd_/)
  expect(allText).not.toContain('digest')
})
