// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect } from 'vitest';
import { listFrontendSourceFiles, readShellHtml } from '../helpers/read-frontend-js.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const appShellSrc = listFrontendSourceFiles(join(fixtureRoot, 'frontend/src/app-shell'))
  .sort()
  .map((abs) => readFileSync(abs, 'utf8'))
  .join('\n');
const indexHtml = [readShellHtml(), appShellSrc].join('\n');
const settingsDialogJs = appShellSrc;

describe('Settings Knowledge Hidden files', () => {
  it('uses Settings Knowledge Hidden files tab', () => {
    expect(indexHtml).toMatch(/data-panel="knowledge">Knowledge</);
    expect(indexHtml).toMatch(/id="settings-panel-knowledge"/);
    expect(indexHtml).toMatch(/data-tab="hidden"[^>]*>Hidden files</);
    expect(indexHtml).toMatch(/id="settings-kb-hide-pattern"/);
    expect(indexHtml).not.toMatch(/id="knowledge-root-dialog"/);
  });

  it('shows hint example \\.xxx$', () => {
    expect(indexHtml).toMatch(/\\\.xxx\$|\\\\\.xxx\$/);
  });

  it('Settings open loads saved pattern into the input', () => {
    expect(settingsDialogJs).toMatch(/function syncKbHidePatternInput\(/);
    expect(settingsDialogJs).toMatch(/getKbHidePattern\(\)/);
  });
});
