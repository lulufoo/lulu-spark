// @vitest-environment jsdom
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { readShellHtml } from '../helpers/read-frontend-js.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

describe('header Sync retired', () => {
  it('removes the header Sync menu and header-sync command', () => {
    expect(existsSync(join(repoRoot, 'frontend/src/app-shell/commands/header-sync.ts'))).toBe(
      false,
    );
    const shell = readFileSync(join(repoRoot, 'frontend/src/shell.tsx'), 'utf8');
    const boot = readFileSync(join(repoRoot, 'frontend/src/boot.ts'), 'utf8');
    const html = readShellHtml();
    expect(shell).not.toContain('btn-sync-menu');
    expect(shell).not.toContain('⇕ Sync');
    expect(shell).not.toContain('btn-push-index');
    expect(shell).not.toContain('btn-pull');
    expect(shell).not.toContain('btn-local-refresh');
    expect(html).not.toContain('btn-sync-menu');
    expect(boot).not.toMatch(/\binitHeaderSync\b/);
    expect(boot).not.toMatch(/\bpullProject\b/);
  });
});
