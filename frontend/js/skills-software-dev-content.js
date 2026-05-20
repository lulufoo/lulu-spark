/** Software Dev Skills dialog content (see tech-doc / lulu-dev-skills README). */
export const softwareDevSkillsContent = {
  title: '✦ Software Dev Skills',
  groups: [
    {
      name: '工具类',
      url: 'https://github.com/lulufoo/lulu-dev-skills/tree/main/sync-rules',
      items: [
        {
          cmd: 'sync-rules',
          name: '规则多平台同步',
          desc: '「同步规则」、sync rules、规则同步；用 `gh` 从 GitHub 拉取配置并写入 Cursor / Claude Code / VS Code',
        },
      ],
    },
    {
      name: '质量与缺陷分析',
      url: 'https://github.com/lulufoo/lulu-dev-skills/tree/main/bug-analysis',
      items: [
        {
          cmd: 'bug-analysis',
          name: 'Bug 分析',
          desc: '缺陷排查、根因分析、调查异常或失败行为',
        },
      ],
    },
    {
      name: '规则守卫',
      url: 'https://github.com/lulufoo/lulu-dev-skills/tree/main/cursor-rule-guard',
      items: [
        {
          cmd: 'cursor-rule-guard',
          name: 'Cursor 规则守卫',
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
          desc: '开发工作流总览；`configure` 拉取 `workflow-config.json`',
        },
        {
          cmd: 'product-doc-workflow',
          name: '产品文档',
          desc: 'Plan；PRD/PDQA；会话路径 `.cache/lulu-dev-workflow/product/<conv_id>/revision{N}/`',
        },
        {
          cmd: 'tech-doc-workflow',
          name: '技术方案',
          desc: 'Plan；E1/E2/E3 评估',
        },
        {
          cmd: 'work-order-workflow',
          name: '施工单',
          desc: 'Plan；task-list、TWCA、WOQA',
        },
        {
          cmd: 'code-workflow',
          name: 'TDD 实现',
          desc: 'Agent；Red/Green/Refactor',
        },
      ],
    },
  ],
}
