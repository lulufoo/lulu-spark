import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { readFrontendJs, readMainSource, readShellHtml } from '../helpers/read-frontend-js.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function readMain() {
  return readMainSource();
}

describe('home-entry hub retired (T-hub / T-read-later)', () => {
  it('boot does not mount the bottom-right home-entry shell', () => {
    const source = readMain();
    expect(source).not.toMatch(/mountHomeEntryShell\s*\(/);
    expect(source).not.toMatch(/from\s+['"]\.\/home-entry-shell\/shell/);
  });

  it('boot does not register createReadLaterContentAdapter', () => {
    const source = readMain();
    expect(source).not.toMatch(/createReadLaterContentAdapter/);
    expect(source).not.toMatch(/from\s+['"].*read-later\/ui\/assistant/);
  });

  it('boot still registers #/read-later via mountReadLaterRoute', () => {
    const source = readMain();
    expect(source).toMatch(/function mountReadLaterRoute/);
    expect(source).toMatch(
      /['"]read-later['"]:\s*wrapRouteMount\s*\(\s*['"]read-later['"]\s*,\s*mountReadLaterRoute/,
    );
  });

  it('removes the hub preview module and keeps the read-later directory', () => {
    expect(existsSync(join(repoRoot, 'frontend/src/read-later/ui/assistant.tsx'))).toBe(
      false,
    );
    expect(existsSync(join(repoRoot, 'frontend/src/read-later'))).toBe(true);
    expect(existsSync(join(repoRoot, 'frontend/src/read-later/ui/dialog.tsx'))).toBe(true);
  });

  it('keeps the home Read Later shortcut and shell ReadLaterDialog', () => {
    const home = readFrontendJs('frontend/src/home/page.tsx');
    const shell = readFrontendJs('frontend/src/shell.tsx');
    const html = readShellHtml();
    expect(home).toMatch(/data-home-entry="read-later"/);
    expect(home).toMatch(/goHomeEntry\(\s*['"]read-later['"]/);
    expect(shell).toMatch(/ReadLaterDialog/);
    expect(html).toMatch(/id="read-later-dialog"/);
    expect(html).not.toMatch(/id="read-later-view"/);
  });

  it('does not add a second in-app Read Later list surface', () => {
    const home = readFrontendJs('frontend/src/home/page.tsx');
    expect(home).not.toMatch(/mountReadLaterList/);
    expect(readFileSync(join(repoRoot, 'frontend/src/boot.ts'), 'utf8')).not.toMatch(
      /mountReadLaterList/,
    );
  });
});
