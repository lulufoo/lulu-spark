import { test, expect } from 'vitest'
import { softwareDevSkillsContent } from '../frontend/js/skills-software-dev-content.js'

const WORKFLOW_CMDS = [
  'lulu-dev-workflow',
  'diagnostic',
  'product',
  'tech',
  'work-order',
  'code',
]

test('softwareDev has 4 groups', () => {
  expect(softwareDevSkillsContent.groups).toHaveLength(4)
})

test('third group is 规则守卫', () => {
  expect(softwareDevSkillsContent.groups[2].name).toBe('规则守卫')
  expect(softwareDevSkillsContent.groups[2].items[0].cmd).toBe('cursor-rule-guard')
})

test('研发工作流 group has 6 workflow commands', () => {
  const wf = softwareDevSkillsContent.groups.find((g) =>
    g.name.includes('研发工作流')
  )
  expect(wf).toBeDefined()
  expect(wf.url).toContain('lulu-dev-workflow')
  expect(wf.items.map((i) => i.cmd)).toEqual(WORKFLOW_CMDS)
})

test('all 9 clickable cmds are unique', () => {
  const cmds = softwareDevSkillsContent.groups.flatMap((g) =>
    g.items.map((i) => i.cmd)
  )
  expect(cmds).toHaveLength(9)
  expect(new Set(cmds).size).toBe(9)
})

test('product desc reflects revision path and diagnostic prerequisite', () => {
  const wf = softwareDevSkillsContent.groups.find((g) =>
    g.name.includes('研发工作流')
  )
  const diagnostic = wf.items.find((i) => i.cmd === 'diagnostic')
  const product = wf.items.find((i) => i.cmd === 'product')
  expect(diagnostic.desc).toMatch(/product\/tech/)
  expect(product.desc).toMatch(/revision/)
})
