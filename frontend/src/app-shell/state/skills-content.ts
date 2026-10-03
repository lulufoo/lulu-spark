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
          desc: 'Collect subtitles or an existing transcript as a full conversation, then save as a note.',
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
          desc: 'Download media, run Whisper, and save the full transcript as a note.',
        },
      ],
    },
    {
      name: 'Notes',
      url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/note-task',
      items: [
        {
          cmd: 'note-task',
          name: 'Notes',
          desc: 'Create and read Workbench notes through MCP.',
        },
      ],
    },
  ],
};
