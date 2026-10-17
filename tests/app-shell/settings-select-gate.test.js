import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const dir = join(repoRoot, 'frontend/src/app-shell/ui/settings');
const MIRRORED = ['settings-llm-engine', 'settings-mcp-channel', 'settings-mcp-ticket-channel', 'settings-theme'];

describe('Settings dropdowns share one look (the Project dropdown)', () => {
  const files = readdirSync(dir).filter((f) => f.endsWith('.tsx'));

  it('every native <select> in Settings is one that list-select.ts mirrors into the shared dropdown', () => {
    for (const file of files) {
      const src = readFileSync(join(dir, file), 'utf8');
      for (const m of src.matchAll(/<select\b[^>]*?id="([^"]+)"/g)) {
        expect(MIRRORED, `${file}: ${m[1]}`).toContain(m[1]);
      }
      const total = (src.match(/<select\b/g) || []).length;
      const withId = (src.match(/<select\b[^>]*?id="/g) || []).length;
      expect(total, `${file} has a <select> without a mirrored id`).toBe(withId);
    }
  });

  it('the workspace add and edit forms use the shared Select', () => {
    for (const file of ['notes-workspaces-add.tsx', 'notes-workspaces-edit.tsx']) {
      const src = readFileSync(join(dir, file), 'utf8');
      expect(src).toMatch(/<Select\b/);
      expect(src).not.toMatch(/<select\b/);
    }
  });

  it('the mirror uses the shared classes, not a Settings-only look', () => {
    const src = readFileSync(join(dir, 'list-select.ts'), 'utf8');
    expect(src).toMatch(/pickerClass: 'ui-select'/);
    expect(src).toMatch(/'ui-select-host'/);
    expect(src).not.toMatch(/settings-select|settings-list-select/);
    for (const id of MIRRORED) expect(src).toContain(`'${id}'`);
  });
});
