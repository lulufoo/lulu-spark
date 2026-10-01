import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { readFrontendJs, readMainSource } from '../helpers/read-frontend-js.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

describe('Builder frontend module retired (T-builder)', () => {
  it('removes frontend/src/builders and its boot adapter registration', () => {
    expect(existsSync(join(repoRoot, 'frontend/src/builders'))).toBe(false);
    const mainJs = readMainSource();
    expect(mainJs).not.toMatch(/createBuildersContentAdapter/);
    expect(mainJs).not.toMatch(/from\s+['"].*builders\//);
    expect(mainJs).not.toMatch(/\.register\(\s*['"]builders['"]/);
  });

  it('does not change the /read-later route or Host jot create path', () => {
    const mainJs = readMainSource();
    expect(mainJs).toMatch(/function mountReadLaterRoute/);
    expect(mainJs).toMatch(
      /['"]read-later['"]:\s*wrapRouteMount\s*\(\s*['"]read-later['"]\s*,\s*mountReadLaterRoute/,
    );
    const createSrc = readFrontendJs('frontend/src/notes/commands/viewer/create.ts');
    expect(createSrc).toMatch(
      /api\.createNote\(\s*\{\s*body:\s*trimmed\s*,\s*source_type:\s*['"]jot['"]\s*\}\s*\)/,
    );
    expect(existsSync(join(repoRoot, 'frontend/src/read-later'))).toBe(true);
  });

  it('does not change MCP create_note invoke mapping', () => {
    const writeMap = readFileSync(
      join(repoRoot, 'frontend/src/host/writeApiInvokeMap.ts'),
      'utf8',
    );
    expect(writeMap).toMatch(/create_note/);
  });
});
