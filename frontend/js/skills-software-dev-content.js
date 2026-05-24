/** Lulu Dev Skills dialog content (see lulu-dev-skills README). */
export const softwareDevSkillsContent = {
  title: '✦ Lulu Dev Skills',
  groups: [
    {
      name: '工具类',
      url: 'https://github.com/lulufoo/lulu-dev-skills/tree/main/lulu-sync-rules',
      items: [
        {
          cmd: 'lulu-sync-rules',
          name: '规则多平台同步',
          desc: '「同步规则」、sync rules、规则同步；用 `gh` 从 GitHub 拉取配置并写入 Cursor / Claude Code / VS Code',
        },
      ],
    },
    {
      name: '质量与缺陷分析',
      url: 'https://github.com/lulufoo/lulu-dev-skills/tree/main/lulu-bug-analysis',
      items: [
        {
          cmd: 'lulu-bug-analysis',
          name: 'Bug 分析',
          desc: '缺陷排查、根因分析、调查异常或失败行为',
        },
      ],
    },
    {
      name: '规则守卫',
      url: 'https://github.com/lulufoo/lulu-dev-skills/tree/main/lulu-rule-guard',
      items: [
        {
          cmd: 'lulu-rule-guard',
          name: '规则守卫',
          desc: '初始化或管理规则守卫、配置 preToolUse 钩子、要求先读后写',
        },
      ],
    },
    {
      name: '研发工作流（lulu-dev-workflow）',
      url: 'https://github.com/lulufoo/lulu-dev-skills/tree/main/lulu-dev-workflow',
      items: [
        {
          cmd: 'lulu-dev-workflow',
          name: '框架总览',
          desc: '开发工作流、dev workflow；`configure` 拉取 `workflow-config.json`',
        },
        {
          cmd: 'diagnostic',
          name: '决策诊断',
          desc: 'Plan；product/tech 强制前置；DDF 六节点（Q/E/D/X/R/V），输出 decision-doc',
        },
        {
          cmd: 'product',
          name: '产品文档',
          desc: 'Plan；PRD/spec、PDQA、ReadyForDelivery；`.cache/.../product/<conv_id>/revision{N}/`',
        },
        {
          cmd: 'tech',
          name: '技术方案',
          desc: 'Plan；tech-doc、E1/E2/E3 评估、tech delivered',
        },
        {
          cmd: 'work-order',
          name: '施工单',
          desc: 'Plan；task-list、TWCA、WOQA',
        },
        {
          cmd: 'code',
          name: 'TDD 实现',
          desc: 'Agent；Red/Green/Refactor；从 Delivered 技术文档或施工单进入编码',
        },
      ],
    },
  ],
}
