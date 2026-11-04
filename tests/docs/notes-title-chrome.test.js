import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const appCss = readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');

function body(selector) {
  const escaped = selector.replace(/[.:()[\]>+*#]/g, '\\$&').replace(/\s+/g, '\\s+');
  const match = new RegExp(`(?:^|\\n)\\s*${escaped}\\s*\\{([^}]*)\\}`).exec(appCss);
  return match ? match[1] : null;
}

describe('notes list and document titles share one chrome', () => {
  it('keeps the document header the same height and padding as the list toolbar', () => {
    expect(body('.notes-page-toolbar')).toMatch(/padding:\s*20px 24px 16px/);
    expect(body('#md-header.viewer-header')).toMatch(/padding:\s*20px 24px 16px/);
    expect(body('#md-header.viewer-header')).toMatch(/min-height:\s*72px/);
    expect(body('.notes-search-btn')).toMatch(/min-height:\s*36px/);
  });

  it('paints the document title with the list heading size and color', () => {
    const list = body('.notes-page-toolbar #date-heading');
    const doc = body('#md-header .viewer-panel-title');
    expect(list).toMatch(/font-size:\s*14px/);
    expect(list).toMatch(/color:\s*var\(--fg-default\)/);
    expect(doc).toMatch(/font-size:\s*14px/);
    expect(doc).toMatch(/color:\s*var\(--fg-default\)/);
    expect(doc).toMatch(/letter-spacing:\s*0/);
    expect(body('.viewer-panel-crumb-link')).toMatch(/color:\s*inherit/);
    expect(body('.viewer-panel-crumb-sep')).toMatch(/color:\s*inherit/);
  });
});
