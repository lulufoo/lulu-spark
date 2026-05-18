import { test, expect } from 'vitest'
import { softwareDevSkillsContent } from '../js/skills-software-dev-content.js'

const WORKFLOW_CMDS = [
  'lulu-dev-workflow',
  'product-doc-workflow',
  'tech-doc-workflow',
  'work-order-workflow',
  'code-workflow',
]

test('softwareDev has 4 groups', () => {
  expect(softwareDevSkillsContent.groups).toHaveLength(4)
})

test('third group is 规则守卫', () => {
  expect(softwareDevSkillsContent.groups[2].name).toBe('规则守卫')
  expect(softwareDevSkillsContent.groups[2].items[0].cmd).toBe('cursor-rule-guard')
})

test('研发工作流 group has 5 workflow commands', () => {
  const wf = softwareDevSkillsContent.groups.find((g) =>
    g.name.includes('研发工作流')
  )
  expect(wf).toBeDefined()
  expect(wf.url).toContain('lulu-dev-workflow')
  expect(wf.items.map((i) => i.cmd)).toEqual(WORKFLOW_CMDS)
})

test('all 8 clickable cmds are unique', () => {
  const cmds = softwareDevSkillsContent.groups.flatMap((g) =>
    g.items.map((i) => i.cmd)
  )
  expect(cmds).toHaveLength(8)
  expect(new Set(cmds).size).toBe(8)
})

test('product-doc-workflow desc reflects revision path, not evaluate snapshot', () => {
  const wf = softwareDevSkillsContent.groups.find((g) =>
    g.name.includes('研发工作流')
  )
  const item = wf.items.find((i) => i.cmd === 'product-doc-workflow')
  expect(item.desc).toMatch(/revision/)
  expect(item.desc).not.toMatch(/evaluate.*product-doc.*快照/i)
})
