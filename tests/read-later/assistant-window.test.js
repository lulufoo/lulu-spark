import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const libRs = readFileSync(join(repoRoot, 'src-tauri/src/lib.rs'), 'utf8');
const defaultCapabilityPath = join(
  repoRoot,
  'src-tauri/capabilities/default.json',
);

const retiredCarriers = [
  {
    html: 'frontend/read-later-assistant.html',
    js: 'frontend/src/read-later/ui/assistant.tsx',
    capability: 'src-tauri/capabilities/read-later-assistant.json',
    window: 'read-later-assistant',
    adapter: /createReadLaterContentAdapter/,
  },

];

describe('retired independent assistant windows', () => {
  it('retires HTML window carriers; keeps remaining shell content modules', () => {
    for (const item of retiredCarriers) {
      expect(existsSync(join(repoRoot, item.html)), `${item.html} retired`).toBe(
        false,
      );
      expect(existsSync(join(repoRoot, item.js)), item.js).toBe(true);
      const js = readFileSync(join(repoRoot, item.js), 'utf8');
      expect(js).toMatch(item.adapter);
    }
    expect(existsSync(join(repoRoot, 'frontend/todo-task-assistant.html'))).toBe(false);
    expect(existsSync(join(repoRoot, 'frontend/src/todo-task'))).toBe(false);
    expect(existsSync(join(repoRoot, 'src-tauri/capabilities/todo-task-assistant.json'))).toBe(false);
  });

  it('retires independent-window helpers and HTML entries from lib.rs', () => {
    expect(libRs).not.toMatch(/READ_LATER_ASSISTANT_LABEL/);
    expect(libRs).not.toMatch(/fn create_read_later_assistant_window\s*\(/);
    expect(libRs).not.toMatch(/fn read_later_assistant_entry_path\s*\(/);
    expect(libRs).not.toMatch(/fn read_later_assistant_spec\s*\(/);
    expect(libRs).not.toMatch(/struct ReadLaterAssistantWindowSpec/);
    expect(libRs).not.toMatch(/WebviewUrl::App\(\s*"read-later-assistant\.html"/);
    expect(libRs).not.toMatch(/"read-later-assistant\.html"/);
    expect(libRs).not.toMatch(/"todo-task-assistant\.html"/);
  });

  it('retires orphan capability islands; main keeps default permissions', () => {
    for (const item of retiredCarriers) {
      expect(
        existsSync(join(repoRoot, item.capability)),
        `${item.capability} retired`,
      ).toBe(false);
    }
    expect(existsSync(defaultCapabilityPath), 'capabilities/default.json').toBe(
      true,
    );
    const capability = JSON.parse(readFileSync(defaultCapabilityPath, 'utf8'));
    expect(capability.identifier).toBe('default');
    expect(capability.windows).toContain('main');
    expect(capability.windows).not.toContain('read-later-assistant');
    expect(capability.windows).not.toContain('todo-task-assistant');

    const capFiles = readdirSync(join(repoRoot, 'src-tauri/capabilities')).filter(
      (name) => name.endsWith('.json'),
    );
    for (const name of capFiles) {
      const cap = JSON.parse(
        readFileSync(join(repoRoot, 'src-tauri/capabilities', name), 'utf8'),
      );
      const windows = Array.isArray(cap.windows) ? cap.windows : [];
      for (const window of ['read-later-assistant', 'todo-task-assistant']) {
        if (windows.includes(window) && !windows.includes('main')) {
          expect.fail(
            `${name} is a permission island for retired ${window} window`,
          );
        }
      }
    }
  });
});
