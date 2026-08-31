import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const DOC_PATH = join(repoRoot, 'docs/archive/knowledge-mcp.md');
const IDE_URL_LITERAL = 'http://127.0.0.1:<mcp_port>/mcp/cursor_ide';

describe('external Cursor IDE mcp.json convention (tech-doc T7 / t3)', () => {
  it('docs/archive/knowledge-mcp.md documents IDE HTTP URL for cursor_ide slot', () => {
    expect(existsSync(DOC_PATH), 'missing docs/archive/knowledge-mcp.md').toBe(true);
    const doc = readFileSync(DOC_PATH, 'utf8');
    expect(doc).toContain(IDE_URL_LITERAL);
    expect(doc).toMatch(/mcp\.json/);
    expect(doc).toMatch(/cursor_ide/);
    expect(doc).toMatch(/mcp_port|MCP_PORT/);
  });

  it('npm test includes cursor-ide mcp.json convention test', () => {
    const pkg = JSON.parse(readFileSync(join(repoRoot, 'package.json'), 'utf8'));
    expect(pkg.scripts.test).toMatch(/vitest run --dir tests/);
  });
});
