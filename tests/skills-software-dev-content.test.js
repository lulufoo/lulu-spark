import { test, expect } from 'vitest'
import { softwareDevSkillsContent } from '../frontend/js/skills-software-dev-content.js'

const WORKFLOW_CMDS = [
  'lulu-dev-workflow',
  'diagnostic',
  'product-diagnostic',
  'product-plan',
  'tech-diagnostic',
  'tech-plan',
  'tech-work-order',
  'tech-code',
]

test('softwareDev has 4 groups', () => {
  expect(softwareDevSkillsContent.groups).toHaveLength(4)
})

test('third group is 规则守卫', () => {
  expect(softwareDevSkillsContent.groups[2].name).toBe('规则守卫')
  expect(softwareDevSkillsContent.groups[2].items[0].cmd).toBe('lulu-rule-guard')
})

test('研发工作流 group has 8 workflow commands', () => {
  const wf = softwareDevSkillsContent.groups.find((g) =>
    g.name.includes('研发工作流')
  )
  expect(wf).toBeDefined()
  expect(wf.url).toContain('lulu-dev-workflow')
  expect(wf.items.map((i) => i.cmd)).toEqual(WORKFLOW_CMDS)
})

test('all 11 clickable cmds are unique', () => {
  const cmds = softwareDevSkillsContent.groups.flatMap((g) =>
    g.items.map((i) => i.cmd)
  )
  expect(cmds).toHaveLength(11)
  expect(new Set(cmds).size).toBe(11)
})

test('workflow items align with SKILL.md names and have desc tooltips', () => {
  const wf = softwareDevSkillsContent.groups.find((g) =>
    g.name.includes('研发工作流')
  )
  const techDiagnostic = wf.items.find((i) => i.cmd === 'tech-diagnostic')
  const productPlan = wf.items.find((i) => i.cmd === 'product-plan')
  expect(techDiagnostic.name).toBe('技术决策诊断')
  expect(productPlan.name).toBe('产品文档')
  expect(techDiagnostic.desc).toMatch(/DDF/)
  expect(productPlan.desc).toMatch(/PDQA/)
  wf.items.forEach((item) => {
    expect(item.desc).toBeTruthy()
  })
})
