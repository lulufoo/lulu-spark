// @vitest-environment node
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const root = join(dirname(fileURLToPath(import.meta.url)), '../..');
const css = readFileSync(join(root, 'frontend/app.css'), 'utf8');
const page = readFileSync(join(root, 'frontend/src/home/page.tsx'), 'utf8');

function rulesFor(selector) {
  const out = [];
  const re = /([^{}]+)\{([^}]*)\}/g;
  for (const m of css.matchAll(re)) {
    const selectors = m[1].split(',').map((x) => x.trim());
    if (selectors.includes(selector)) out.push(m[2]);
  }
  return out;
}

describe('sidebar section labels (Notion style)', () => {
  for (const selector of ['.home-chat-nav-label']) {
    it(`${selector} is small grey plain text`, () => {
      const rules = rulesFor(selector);
      expect(rules.length).toBeGreaterThan(0);
      const all = rules.join('\n');
      expect(all).not.toMatch(/uppercase/);
      expect(all).not.toMatch(/letter-spacing/);
      expect(all).toMatch(/font-size: 12px/);
      expect(all).toMatch(/font-weight: 400/);
      expect(all).toMatch(/color: var\(--fg-subtle\)/);
    });
  }

  it('uses one variable for the label row height', () => {
    expect(css).toMatch(/--home-nav-label-h: 28px;/);
    expect(rulesFor('.home-chat-nav-label').join('\n')).toMatch(/height: var\(--home-nav-label-h\)/);
  });

  it('keeps the Workspace label and the new-chat button, with no Chats label', () => {
    expect(page).toMatch(/home-chat-nav-label">Workspace</);
    expect(page).not.toMatch(/home-chat-sessions-title/);
    expect(page).toMatch(/data-role="new-session"/);
  });
});
