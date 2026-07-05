import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const libRs = readFileSync(join(repoRoot, 'src-tauri/src/lib.rs'), 'utf8');
const assistantCapability = JSON.parse(
  readFileSync(
    join(repoRoot, 'src-tauri/capabilities/read-later-assistant.json'),
    'utf8',
  ),
);

describe('read-later-assistant WebviewWindow (lib.rs)', () => {
  it('defines create_read_later_assistant_window helper', () => {
    expect(libRs).toMatch(/fn create_read_later_assistant_window\s*\(/);
  });

  it('creates window with WebviewWindowBuilder::new (not from_config)', () => {
    const fnBody = libRs.match(
      /fn create_read_later_assistant_window[\s\S]*?^}/m,
    )?.[0];
    expect(fnBody, 'create_read_later_assistant_window body').toBeTruthy();
    expect(fnBody).toMatch(/WebviewWindowBuilder::new/);
    expect(fnBody).not.toMatch(/WebviewWindowBuilder::from_config/);
  });

  it('uses read-later-assistant label and always_on_top', () => {
    const fnBody = libRs.match(
      /fn create_read_later_assistant_window[\s\S]*?^}/m,
    )?.[0];
    expect(fnBody).toMatch(/read-later-assistant/);
    expect(fnBody).toMatch(/always_on_top\s*\(\s*true\s*\)/);
  });

  it('loads read-later-assistant.html via WebviewUrl::App', () => {
    const fnBody = libRs.match(
      /fn create_read_later_assistant_window[\s\S]*?^}/m,
    )?.[0];
    expect(fnBody).toMatch(/WebviewUrl::App/);
    expect(fnBody).toMatch(/read-later-assistant\.html/);
  });

  it('setup does not create separate assistant WebviewWindow', () => {
    const setupBody = libRs.match(/\.setup\s*\(\s*\|app\|[\s\S]*?Ok\s*\(\s*\(\s*\)\s*\)/m)?.[0];
    expect(setupBody, 'setup closure').toBeTruthy();
    expect(setupBody).not.toMatch(/create_read_later_assistant_window\s*\(/);
  });

  it('guards against duplicate assistant windows (singleton)', () => {
    const fnBody = libRs.match(
      /fn create_read_later_assistant_window[\s\S]*?^}/m,
    )?.[0];
    expect(fnBody).toMatch(/get_webview_window/);
  });

  it('keeps assistant alive when main window closes', () => {
    expect(libRs).toMatch(/CloseRequested/);
    expect(libRs).toMatch(/prevent_close|hide\s*\(/);
  });

  it('grants read-api and opener to read-later-assistant window', () => {
    expect(assistantCapability.windows).toContain('read-later-assistant');
    expect(assistantCapability.permissions).toContain('read-api');
    expect(assistantCapability.permissions).toContain('opener:default');
  });
});
