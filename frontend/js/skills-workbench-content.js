/** Lulu Workbench Skills dialog content (see lulu-workbench-skills README). */
export const workbenchSkillsContent = {
  title: '✦ Lulu Workbench Skills',
  groups: [
    {
      name: '对话回顾',
      url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/dialogue-summary',
      items: [
        {
          cmd: 'dialogue-summary',
          name: '过程回顾',
          desc: '复盘对话过程、梳理决策脉络，并归档为 Workbench 主题文档',
        },
      ],
    },
    {
      name: '对话原文归档',
      url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/dialogue-archive',
      items: [
        {
          cmd: 'dialogue-archive',
          name: '逐轮归档',
          desc: '按轮次保留对话原文，归一化保存到 Workbench 文档',
        },
      ],
    },
    {
      name: '主题线整理',
      url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-line',
      items: [
        {
          cmd: 'theme-line',
          name: 'Transcript 主题线',
          desc: '将视频或访谈 transcript 按主题重组为时间线大纲',
        },
      ],
    },
    {
      name: '网页采集',
      url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-fetch',
      items: [
        {
          cmd: 'theme-fetch',
          name: '文章采集',
          desc: '抓取网页或公众号文章，格式化后保存为可读文档',
        },
      ],
    },
    {
      name: '主题文档归档',
      url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-archive',
      items: [
        {
          cmd: 'theme-archive',
          name: '文档落盘',
          desc: '将已格式化文档写入 Workbench raw 目录并更新索引',
        },
      ],
    },
    {
      name: 'Plan 任务',
      url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/plan-task',
      items: [
        {
          cmd: 'plan-task',
          name: '任务计划',
          desc: '经 Workbench MCP 创建、查询和维护 plan 任务树',
        },
      ],
    },
  ],
}
