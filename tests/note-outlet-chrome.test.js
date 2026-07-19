import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const indexHtml = readFileSync(join(repoRoot, 'frontend/index.html'), 'utf8');

/** Extract outermost element with id, balanced for nested same-tag children. */
function extractById(html, id) {
  const openRe = new RegExp(`<([a-zA-Z0-9]+)([^>]*\\bid="${id}"[^>]*)>`, 'i');
  const open = openRe.exec(html);
  expect(open, `missing element #${id}`).toBeTruthy();
  const tag = open[1];
  const start = open.index;
  const afterOpen = start + open[0].length;
  const voidish = /^(input|br|hr|img|meta|link)$/i.test(tag);
  if (voidish || /\/>$/.test(open[0])) {
    return html.slice(start, afterOpen);
  }
  let depth = 1;
  const token = new RegExp(`</?${tag}\\b[^>]*>`, 'gi');
  token.lastIndex = afterOpen;
  let m;
  while ((m = token.exec(html)) !== null) {
    if (m[0].startsWith(`</`)) depth -= 1;
    else if (!/\/>$/.test(m[0])) depth += 1;
    if (depth === 0) {
      return html.slice(start, m.index + m[0].length);
    }
  }
  throw new Error(`unclosed #${id}`);
}

describe('T4 note outlet hosts viewer chrome (tech-doc T4 / index.html)', () => {
  it('keeps workbench shell: sidebar + #main', () => {
    expect(indexHtml).toMatch(/id="sidebar"/);
    expect(indexHtml).toMatch(/<main[^>]*\bid="main"/);
  });

  it('places #note-outlet inside #main as peer to #doc-list', () => {
    const main = extractById(indexHtml, 'main');
    expect(main).toMatch(/id="doc-list"/);
    expect(main).toMatch(/id="note-outlet"/);
    const docListAt = main.indexOf('id="doc-list"');
    const outletAt = main.indexOf('id="note-outlet"');
    expect(outletAt).toBeGreaterThan(docListAt);
  });

  it('moves #md-panel tree into #note-outlet (not body-level #md-modal)', () => {
    const outlet = extractById(indexHtml, 'note-outlet');
    expect(outlet).toMatch(/id="md-panel"/);
    expect(outlet).toMatch(/id="md-body"/);
    expect(outlet).toMatch(/id="md-edit-area"/);
    expect(outlet).toMatch(/id="md-header"/);
    expect(outlet).toMatch(/id="knowledge-panel"/);

    const modal = extractById(indexHtml, 'md-modal');
    expect(modal).not.toMatch(/id="md-panel"/);
    expect(modal).not.toMatch(/id="md-body"/);
  });

  it('keeps a single chrome id surface (no parallel second md-panel tree)', () => {
    const panelMatches = indexHtml.match(/id="md-panel"/g) || [];
    const bodyMatches = indexHtml.match(/id="md-body"/g) || [];
    const editMatches = indexHtml.match(/id="md-edit-area"/g) || [];
    expect(panelMatches).toHaveLength(1);
    expect(bodyMatches).toHaveLength(1);
    expect(editMatches).toHaveLength(1);
  });
});
