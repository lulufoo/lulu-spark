/** Lulu Workbench Skills dialog content (see lulu-workbench-skills SKILL.md). */
export const workbenchSkillsContent = {
  title: '✦ Lulu Workbench Skills',
  groups: [
    {
      name: 'Dialogue summary',
      url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/dialogue-summary',
      items: [
        {
          cmd: 'dialogue-summary',
          name: 'Dialogue summary',
          desc: 'Turn the conversation into a standalone process summary and archive it when needed.',
        },
      ],
    },
    {
      name: 'Dialogue archive',
      url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/dialogue-archive',
      items: [
        {
          cmd: 'dialogue-archive',
          name: 'Dialogue archive',
          desc: 'Slice nodes and archive the original text; local-md writes .cache only.',
        },
      ],
    },
    {
      name: 'Full conversation',
      url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-line',
      items: [
        {
          cmd: 'theme-line',
          name: 'Full conversation',
          desc: 'Collect subtitles or an existing transcript as a full conversation for theme-archive.',
        },
      ],
    },
    {
      name: 'Web fetch',
      url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-fetch',
      items: [
        {
          cmd: 'theme-fetch',
          name: 'Web fetch',
          desc: 'Fetch a web or WeChat article, format it, and hand it to theme-archive.',
        },
      ],
    },
    {
      name: 'Video transcription',
      url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-transcribe',
      items: [
        {
          cmd: 'theme-transcribe',
          name: 'Video transcription',
          desc: 'Download media, run Whisper, and hand the full transcript to theme-archive.',
        },
      ],
    },
    {
      name: 'Theme archive',
      url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-archive',
      items: [
        {
          cmd: 'theme-archive',
          name: 'Theme archive',
          desc: 'Write the formatted document to raw and update the index.',
        },
      ],
    },
    {
      name: 'Todo tasks',
      url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/todo-task',
      items: [
        {
          cmd: 'todo-task',
          name: 'Todo tasks',
          desc: 'Create, query, and maintain Todo task trees through Workbench MCP.',
        },
      ],
    },
  ],
}
