import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const appCss = readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');

function body(selector) {
  const escaped = selector.replace(/[.:()[\]>+*]/g, '\\$&').replace(/\s+/g, '\\s+');
  const match = new RegExp(`(?:^|\\n)${escaped}\\s*\\{([^}]*)\\}`).exec(appCss);
  return match ? match[1] : null;
}

describe('project row Delete is lighter than Edit', () => {
  it('rests in grey at weight 400, above the shared 600 button rule in specificity', () => {
    expect(body('.notes-cat-row-actions button')).toMatch(/font-weight:\s*600/);
    const rest = body('.notes-cat-row-actions .notes-cat-row-delete');
    expect(rest).toMatch(/color:\s*var\(--fg-muted\)/);
    expect(rest).toMatch(/font-weight:\s*400/);
  });

  it('turns danger red on hover and keyboard focus without changing weight', () => {
    const hover = appCss.match(
      /\.notes-cat-row-delete:hover:not\(:disabled\),\s*\.notes-cat-row-delete:focus-visible\s*\{([^}]*)\}/,
    );
    expect(hover?.[1]).toMatch(/color:\s*var\(--fg-danger\)/);
    expect(hover?.[1]).not.toMatch(/font-weight/);
  });

  it('leaves Edit as the weight-600 accent link', () => {
    expect(body('.notes-cat-row-edit')).toMatch(/color:\s*var\(--fg-accent\)/);
    expect(body('.notes-cat-row-edit')).not.toMatch(/font-weight/);
  });
});
