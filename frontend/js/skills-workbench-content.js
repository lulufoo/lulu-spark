/** Lulu Workbench Skills dialog content (see lulu-workbench-skills SKILL.md). */
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
          desc: '将对话整理为可独立阅读的过程总结，并按需归档',
        },
      ],
    },
    {
      name: '对话原文归档',
      url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/dialogue-archive',
      items: [
        {
          cmd: 'dialogue-archive',
          name: '原文归档',
          desc: '节点切片后按原文归档；local-md 仅落 .cache',
        },
      ],
    },
    {
      name: '完整对话整理',
      url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-line',
      items: [
        {
          cmd: 'theme-line',
          name: '完整对话整理',
          desc: '采集字幕或已有稿为完整对话后交给 theme-archive',
        },
      ],
    },
    {
      name: '网页采集',
      url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-fetch',
      items: [
        {
          cmd: 'theme-fetch',
          name: '网页文章采集',
          desc: '抓取网页或公众号文章，格式化后交给 theme-archive',
        },
      ],
    },
    {
      name: '视频转写',
      url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-transcribe',
      items: [
        {
          cmd: 'theme-transcribe',
          name: '视频转写流水线',
          desc: '下载媒体 + Whisper，生成完整逐字稿后交给 theme-archive',
        },
      ],
    },
    {
      name: '主题文档归档',
      url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-archive',
      items: [
        {
          cmd: 'theme-archive',
          name: '文档归档',
          desc: '将已格式化文档写入 raw 目录并更新索引',
        },
      ],
    },
    {
      name: 'Todo 任务',
      url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/todo-task',
      items: [
        {
          cmd: 'todo-task',
          name: 'Todo 任务',
          desc: '经 Workbench MCP 创建、查询和维护 Todo 任务树',
        },
      ],
    },
  ],
}
