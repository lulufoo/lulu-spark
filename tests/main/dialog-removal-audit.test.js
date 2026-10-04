import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { readShellHtml } from '../helpers/read-frontend-js.js';
import { readMainSource, readNotesViewerSource } from '../helpers/read-frontend-js.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const indexHtml = [
  readShellHtml(),
  readFileSync(join(repoRoot, 'frontend/src/notes/page.tsx'), 'utf8'),
].join('\n');
const appCss = readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');
const viewerJs = readNotesViewerSource();
const mainJs = readMainSource();

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

describe('T9 Dialog removal audit (tech-doc T9 / SK-P3)', () => {
  it('keeps a single spark note chrome tree under #note-outlet', () => {
    expect(indexHtml).toMatch(/id="note-outlet"/);
    expect(indexHtml).toMatch(/id="md-panel"/);
    expect(indexHtml).toMatch(/id="note-outlet-message"/);
    expect(indexHtml.match(/id="md-panel"/g) || []).toHaveLength(1);
    expect(indexHtml.match(/id="note-outlet"/g) || []).toHaveLength(1);
    expect(indexHtml).not.toMatch(/\bid="md-modal"/);
    expect(indexHtml).not.toMatch(/\bid="md-backdrop"/);
  });

  it('embeds .viewer-panel in #note-outlet (not fixed modal chrome)', () => {
    // Create chrome class is on #note-outlet (viewer.js applyCreateChrome)
    expect(appCss).toMatch(/#note-outlet\.is-create\s+\.viewer-chrome-persisted/);
    expect(appCss).toMatch(/#note-outlet\s+\.viewer-panel/);
    // Embedded fill — must not keep modal-only width clamp as the only .viewer-panel rule
    const panelBlock = appCss.match(/#note-outlet\s+\.viewer-panel\s*\{[^}]+\}/);
    expect(panelBlock, 'missing #note-outlet .viewer-panel block').toBeTruthy();
    expect(panelBlock[0]).toMatch(/width:\s*100%/);
    expect(panelBlock[0]).not.toMatch(/min\(1180px/);
  });

  it('close/dismiss exits via navigateBackToList (no #md-modal display semantics)', () => {
    expect(viewerJs).toMatch(/navigateBackToList/);
    expect(viewerJs).toMatch(/navigateBackToList\s*\(/);
    // Dead Dialog shell must not drive open/close
    expect(viewerJs).not.toMatch(/getElementById\(\s*['"]md-modal['"]\s*\)/);
    expect(viewerJs).not.toMatch(/getElementById\(\s*['"]md-backdrop['"]\s*\)/);
  });

  it('mountSpark does not wipe #md-panel / outlet textContent; message stays in state', () => {
    const start = mainJs.indexOf('function mountSpark');
    expect(start).toBeGreaterThanOrEqual(0);
    const brace = mainJs.indexOf('{', start);
    let depth = 0;
    let end = -1;
    for (let i = brace; i < mainJs.length; i += 1) {
      if (mainJs[i] === '{') depth += 1;
      if (mainJs[i] === '}') {
        depth -= 1;
        if (depth === 0) {
          end = i;
          break;
        }
      }
    }
    const body = mainJs.slice(start, end + 1);
    expect(body).not.toMatch(/outlet\.textContent\s*=\s*message/);
    expect(body).not.toMatch(/outlet\.textContent\s*=\s*['"]{2}/);
    expect(body).toMatch(/note-outlet-message|outletMessage/);
  });
});
