import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

// Independent assistant HTML window carriers retired; shell content lives in *-assistant.js.
const aiAssistantHtmlPath = join(repoRoot, 'frontend/ai-assistant.html');
const readLaterAssistantHtmlPath = join(
  repoRoot,
  'frontend/read-later-assistant.html',
);
const todoTaskAssistantHtmlPath = join(
  repoRoot,
  'frontend/todo-task-assistant.html',
);
const homeHubJs = readFileSync(
  join(repoRoot, 'frontend/js/home-entry-shell/hub.js'),
  'utf8',
);
const readLaterAssistantJs = readFileSync(
  join(repoRoot, 'frontend/js/read-later/assistant.js'),
  'utf8',
);
const todoTaskAssistantJs = readFileSync(
  join(repoRoot, 'frontend/js/todo-task/assistant.js'),
  'utf8',
);

const assistantSources = [
  homeHubJs,
  readLaterAssistantJs,
  todoTaskAssistantJs,
].join('\n');

describe('P2 copy-switch — Assistant pages (tech-doc T5)', () => {
  it('assistant HTML shells are retired', () => {
    expect(existsSync(aiAssistantHtmlPath)).toBe(false);
    expect(existsSync(readLaterAssistantHtmlPath)).toBe(false);
    expect(existsSync(todoTaskAssistantHtmlPath)).toBe(false);
  });

  it('Home chat uses English Assistant copy', () => {
    expect(existsSync(join(repoRoot, 'frontend/js/ai-assistant.js'))).toBe(false);
    expect(homeHubJs).toContain('Chats');
    expect(homeHubJs).toContain('Message…');
    expect(homeHubJs).toContain('Send');
    expect(homeHubJs).toContain('query_binding');
    expect(homeHubJs).toContain('Busy — try again later');
    expect(homeHubJs).toContain('Failed to send');
    expect(homeHubJs).toContain('Chat requires a workspace Binding.');
    expect(homeHubJs).not.toContain('No todo bound');
    expect(homeHubJs).not.toContain('No todo session bound');
    expect(homeHubJs).not.toContain('待办助手');
    expect(homeHubJs).not.toContain('尚未绑定');
    expect(homeHubJs).not.toContain('发送');
  });

  it('read-later-assistant uses table A/B/B2 English copy', () => {
    expect(readLaterAssistantJs).toContain('Read Later');
    expect(readLaterAssistantJs).toContain('No items to read later');
    expect(readLaterAssistantJs).toContain(
      'After saving with the Chrome extension, latest items appear here',
    );
    expect(readLaterAssistantJs).toContain('Temporarily unavailable');
    expect(readLaterAssistantJs).toContain('Recent items');
    expect(readLaterAssistantJs).toContain('aria-label="Next"');
    expect(readLaterAssistantJs).toContain('View all read-later →');
    expect(readLaterAssistantJs).toContain('Loading…');
    expect(readLaterAssistantJs).toContain('Open Read Later assistant');
    expect(readLaterAssistantJs).toContain("toLocaleString('en'");
    expect(readLaterAssistantJs).not.toContain('待读助手');
    expect(readLaterAssistantJs).not.toContain('暂无待读');
    expect(readLaterAssistantJs).not.toContain('加载中');
  });

  it('todo-task-assistant uses table B/B2 English copy', () => {
    expect(todoTaskAssistantJs).toContain('No todos yet');
    expect(todoTaskAssistantJs).toContain(
      'After creating via MCP, latest tasks appear here',
    );
    expect(todoTaskAssistantJs).toContain('Temporarily unavailable');
    expect(todoTaskAssistantJs).toContain('View all →');
    expect(todoTaskAssistantJs).toContain('Loading…');
    expect(todoTaskAssistantJs).toContain('Open Todos');
    expect(todoTaskAssistantJs).toMatch(/\$\{complete\}\/\$\{total\} complete/);
    expect(todoTaskAssistantJs).not.toContain('暂无Todos');
    expect(todoTaskAssistantJs).not.toContain(' 完成');
    expect(todoTaskAssistantJs).not.toContain('加载中');
  });

  it('assistant sources have no user-facing Chinese (excl. comments)', () => {
    const withoutBlockComments = assistantSources.replace(/\/\*[\s\S]*?\*\//g, '');
    const withoutLineComments = withoutBlockComments.replace(/\/\/.*$/gm, '');
    expect(withoutLineComments).not.toMatch(/[\u4e00-\u9fff]/);
  });
});
