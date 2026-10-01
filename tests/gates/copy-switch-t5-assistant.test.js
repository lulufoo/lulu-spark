import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { readFrontendJs } from '../helpers/read-frontend-js.js';

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
const homeHubJs = readFrontendJs('frontend/src/home/hub.tsx');

const assistantSources = [
  homeHubJs,
].join('\n');

describe('P2 copy-switch — Assistant pages (tech-doc T5)', () => {
  it('assistant HTML shells are retired', () => {
    expect(existsSync(aiAssistantHtmlPath)).toBe(false);
    expect(existsSync(readLaterAssistantHtmlPath)).toBe(false);
    expect(existsSync(todoTaskAssistantHtmlPath)).toBe(false);
  });

  it('Home chat uses English Assistant copy', () => {
    expect(existsSync(join(repoRoot, 'frontend/src/ai-assistant.js'))).toBe(false);
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

  it('read-later hub preview module is retired', () => {
    expect(existsSync(join(repoRoot, 'frontend/src/read-later/ui/assistant.tsx'))).toBe(
      false,
    );
  });

  it('todo-task assistant module is removed', () => {
    expect(existsSync(join(repoRoot, 'frontend/src/todo-task'))).toBe(false);
    expect(existsSync(todoTaskAssistantHtmlPath)).toBe(false);
  });

  it('assistant sources have no user-facing Chinese (excl. comments)', () => {
    const withoutBlockComments = assistantSources.replace(/\/\*[\s\S]*?\*\//g, '');
    const withoutLineComments = withoutBlockComments.replace(/\/\/.*$/gm, '');
    expect(withoutLineComments).not.toMatch(/[\u4e00-\u9fff]/);
  });
});
