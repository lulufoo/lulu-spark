// @vitest-environment jsdom
/**
 * T5: document-level FAB mutual-exclusion listener is retired.
 * Mutex lives inside the home-entry shell; main.js must not keep the old capture listener.
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');

function readMain() {
  return readFileSync(join(repoRoot, 'frontend/js/main.js'), 'utf8');
}

describe('Builders ↔ assistants capture-phase mutex retired (main.js T5)', () => {
  it('main.js no longer contains the document-level FAB mutex closest checks', () => {
    const source = readMain();
    expect(source).not.toMatch(/closest\(\s*['"]\.rl-assistant-fab['"]\s*\)/);
    expect(source).not.toMatch(/closest\(\s*['"]\.pt-assistant-fab['"]\s*\)/);
    expect(source).not.toMatch(/closest\(\s*['"]\.note-assistant-fab['"]\s*\)/);
    expect(source).not.toMatch(/closest\(\s*['"]\.builders-entry-fab['"]\s*\)/);
  });

  it('main.js mounts the home-entry shell once instead of four assistant widgets', () => {
    const source = readMain();
    const mounts = source.match(/mountHomeEntryShell\s*\(\s*document\.body\b/g) || [];
    expect(mounts.length).toBe(1);
    expect(source).not.toMatch(
      /mount(?:ReadLater|PlanTask|Note|Builders)AssistantWidget\s*\(\s*document\.body\b/,
    );
  });
});
