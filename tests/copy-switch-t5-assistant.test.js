import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');

// Independent assistant HTML window carriers retired; shell content lives in *-assistant.js.
const aiAssistantHtmlPath = join(repoRoot, 'frontend/ai-assistant.html');
const readLaterAssistantHtmlPath = join(
  repoRoot,
  'frontend/read-later-assistant.html',
);
const planTaskAssistantHtmlPath = join(
  repoRoot,
  'frontend/plan-task-assistant.html',
);
const aiAssistantJs = readFileSync(
  join(repoRoot, 'frontend/js/ai-assistant.js'),
  'utf8',
);
const readLaterAssistantJs = readFileSync(
  join(repoRoot, 'frontend/js/read-later-assistant.js'),
  'utf8',
);
const planTaskAssistantJs = readFileSync(
  join(repoRoot, 'frontend/js/plan-task-assistant.js'),
  'utf8',
);

const assistantSources = [
  aiAssistantJs,
  readLaterAssistantJs,
  planTaskAssistantJs,
].join('\n');

describe('P2 copy-switch — Assistant pages (tech-doc T5)', () => {
  it('assistant HTML shells are retired', () => {
    expect(existsSync(aiAssistantHtmlPath)).toBe(false);
    expect(existsSync(readLaterAssistantHtmlPath)).toBe(false);
    expect(existsSync(planTaskAssistantHtmlPath)).toBe(false);
  });

  it('ai-assistant uses table B2 Assistant branding and copy', () => {
    // Branding lives in shell content module after independent HTML retirement (T4).
    expect(aiAssistantJs).toContain('Assistant');
    expect(aiAssistantJs).toContain('Unbound');
    expect(aiAssistantJs).toContain('Bound');
    expect(aiAssistantJs).toContain('Message…');
    expect(aiAssistantJs).toContain('aria-label="Send"');
    expect(aiAssistantJs).not.toContain('Bound:');
    expect(aiAssistantJs).toContain('query_binding');
    expect(aiAssistantJs).toContain('Tauri invoke unavailable');
    expect(aiAssistantJs).toContain('Working…');
    expect(aiAssistantJs).toContain('Busy — try again later');
    expect(aiAssistantJs).toContain('Failed to send');
    expect(aiAssistantJs).not.toContain('No todo bound');
    expect(aiAssistantJs).not.toContain('No todo session bound');
    expect(aiAssistantJs).not.toContain('待办助手');
    expect(aiAssistantJs).not.toContain('尚未绑定');
    expect(aiAssistantJs).not.toContain('发送');
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

  it('plan-task-assistant uses table B/B2 English copy', () => {
    expect(planTaskAssistantJs).toContain('No todos yet');
    expect(planTaskAssistantJs).toContain(
      'After creating via MCP, latest tasks appear here',
    );
    expect(planTaskAssistantJs).toContain('Temporarily unavailable');
    expect(planTaskAssistantJs).toContain('View all →');
    expect(planTaskAssistantJs).toContain('Loading…');
    expect(planTaskAssistantJs).toContain('Open Todos');
    expect(planTaskAssistantJs).toMatch(/\$\{complete\}\/\$\{total\} complete/);
    expect(planTaskAssistantJs).not.toContain('暂无Todos');
    expect(planTaskAssistantJs).not.toContain(' 完成');
    expect(planTaskAssistantJs).not.toContain('加载中');
  });

  it('assistant sources have no user-facing Chinese (excl. comments)', () => {
    const withoutBlockComments = assistantSources.replace(/\/\*[\s\S]*?\*\//g, '');
    const withoutLineComments = withoutBlockComments.replace(/\/\/.*$/gm, '');
    expect(withoutLineComments).not.toMatch(/[\u4e00-\u9fff]/);
  });
});
