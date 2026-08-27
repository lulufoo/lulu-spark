import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
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

describe('T7 workbench notes Dialog entry removed (tech-doc T7 / index.html)', () => {
  it('removes body-level #md-modal / #md-backdrop Dialog entry', () => {
    expect(indexHtml).not.toMatch(/\bid="md-modal"/);
    expect(indexHtml).not.toMatch(/\bid="md-backdrop"/);
  });

  it('keeps note chrome under #note-outlet inside #main', () => {
    const main = extractById(indexHtml, 'main');
    const outlet = extractById(main, 'note-outlet');
    expect(outlet).toMatch(/id="md-panel"/);
    expect(outlet).toMatch(/id="md-body"/);
    expect(outlet).toMatch(/id="md-edit-area"/);
  });

  it('does not reintroduce KB #kb-md-modal (out of scope)', () => {
    // IV-3: KB parallel shell is out of this path — T7 must not add it back.
    expect(indexHtml).not.toMatch(/\bid="kb-md-modal"/);
  });
});
