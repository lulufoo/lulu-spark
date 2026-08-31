import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const README_PATH = join(repoRoot, 'README.md');
const IDE_URL_LITERAL = 'http://127.0.0.1:<mcp_port>/mcp/cursor_ide';

describe('external Cursor IDE mcp.json convention (tech-doc T7 / t3)', () => {
  it('README documents IDE HTTP URL for cursor_ide slot', () => {
    const doc = readFileSync(README_PATH, 'utf8');
    expect(doc).toContain(IDE_URL_LITERAL);
    expect(doc).toMatch(/mcp\.json/);
    expect(doc).toMatch(/cursor_ide/);
    expect(doc).toMatch(/mcp_port|MCP_PORT|9876/);
  });

  it('npm test includes cursor-ide mcp.json convention test', () => {
    const pkg = JSON.parse(readFileSync(join(repoRoot, 'package.json'), 'utf8'));
    expect(pkg.scripts.test).toMatch(/vitest run --dir tests/);
  });
});
