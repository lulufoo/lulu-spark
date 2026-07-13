// @vitest-environment jsdom
/**
 * T4: extend main.js document capture-phase click listener for
 * Builders ↔ rl/pt/note mutual exclusion.
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, vi } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');

function readMain() {
  return readFileSync(join(repoRoot, 'frontend/js/main.js'), 'utf8');
}

/** Extract the fab-mutex capture-phase click listener from main.js. */
function extractFabMutexListener(source) {
  const needle = "target.closest('.rl-assistant-fab')";
  const hit = source.indexOf(needle);
  if (hit === -1) return null;

  const addStart = source.lastIndexOf('document.addEventListener(', hit);
  if (addStart === -1 || addStart > hit) return null;

  const slice = source.slice(addStart);
  const trueClose = slice.match(/,\s*true\s*,?\s*\)\s*;/);
  if (!trueClose) return null;

  const block = slice.slice(0, trueClose.index + trueClose[0].length);
  const arrow = block.indexOf('(event) =>');
  if (arrow === -1) return null;
  const braceStart = block.indexOf('{', arrow);
  let depth = 0;
  let end = -1;
  for (let i = braceStart; i < block.length; i += 1) {
    if (block[i] === '{') depth += 1;
    else if (block[i] === '}') {
      depth -= 1;
      if (depth === 0) {
        end = i;
        break;
      }
    }
  }
  if (end === -1) return null;
  return {
    full: block,
    body: block.slice(braceStart + 1, end),
    capture: true,
  };
}

function compileListener(body, env) {
  return new Function(
    'Element',
    'readLaterAssistant',
    'planTaskAssistant',
    'noteAssistant',
    'buildersAssistant',
    `return function (event) { ${body} };`,
  )(
    Element,
    env.readLaterAssistant,
    env.planTaskAssistant,
    env.noteAssistant,
    env.buildersAssistant,
  );
}

function makeFab(className) {
  const el = document.createElement('button');
  el.className = className;
  document.body.appendChild(el);
  return el;
}

function branchAfter(body, selector) {
  const re = new RegExp(
    `closest\\(\\s*['"]\\.${selector}['"]\\s*\\)([\\s\\S]*?)(?=closest\\(|$)`,
  );
  return body.match(re)?.[1] ?? '';
}

function closesBuilders(branch) {
  return (
    /buildersAssistant\?\.close\s*\(/.test(branch) ||
    /buildersAssistant\?\.setOpen\(\s*false\s*\)/.test(branch) ||
    /buildersAssistant\.close\s*\(/.test(branch) ||
    /buildersAssistant\.setOpen\(\s*false\s*\)/.test(branch)
  );
}

describe('Builders ↔ assistants capture-phase mutex (main.js T4)', () => {
  it('extends the existing document capture-phase (true) click listener — no parallel strategy', () => {
    const source = readMain();
    const listener = extractFabMutexListener(source);
    expect(listener, 'fab mutex listener not found').not.toBeNull();
    expect(listener.capture).toBe(true);
    // Single fab-mutex listener: one rl-assistant-fab closest check in main.js
    const fabHits = source.match(/closest\(\s*['"]\.rl-assistant-fab['"]\s*\)/g) || [];
    expect(fabHits.length).toBe(1);
    // Builders mutual exclusion lives in that same capture listener body
    expect(listener.body).toMatch(/builders-entry-fab/);
    expect(listener.full).toMatch(/,\s*true\s*,?\s*\)\s*;/);
  });

  it('Builders FAB branch closes readLater / planTask / note via setOpen(false)', () => {
    const { body } = extractFabMutexListener(readMain());
    expect(body).toMatch(/closest\(\s*['"]\.builders-entry-fab['"]\s*\)/);
    const builders = branchAfter(body, 'builders-entry-fab');
    expect(builders).toMatch(/readLaterAssistant\?\.setOpen\(\s*false\s*\)/);
    expect(builders).toMatch(/planTaskAssistant\?\.setOpen\(\s*false\s*\)/);
    expect(builders).toMatch(/noteAssistant\?\.setOpen\(\s*false\s*\)/);
  });

  it('rl / pt / note FAB branches call Builders close or setOpen(false)', () => {
    const { body } = extractFabMutexListener(readMain());
    expect(closesBuilders(branchAfter(body, 'rl-assistant-fab'))).toBe(true);
    expect(closesBuilders(branchAfter(body, 'pt-assistant-fab'))).toBe(true);
    expect(closesBuilders(branchAfter(body, 'note-assistant-fab'))).toBe(true);
  });

  it('uses optional chaining where assistants may be null', () => {
    const { body } = extractFabMutexListener(readMain());
    expect(body).toMatch(/noteAssistant\?\.setOpen\(\s*false\s*\)/);
    expect(body).toMatch(/buildersAssistant\?\.(?:close|setOpen)/);
  });

  it('behavioral: Builders FAB closes other assistants; null noteAssistant does not throw', () => {
    const { body } = extractFabMutexListener(readMain());
    const readLaterAssistant = { setOpen: vi.fn() };
    const planTaskAssistant = { setOpen: vi.fn() };
    const buildersAssistant = { setOpen: vi.fn(), close: vi.fn() };
    const fn = compileListener(body, {
      readLaterAssistant,
      planTaskAssistant,
      noteAssistant: null,
      buildersAssistant,
    });
    const fab = makeFab('builders-entry-fab');
    expect(() => fn({ target: fab })).not.toThrow();
    expect(readLaterAssistant.setOpen).toHaveBeenCalledWith(false);
    expect(planTaskAssistant.setOpen).toHaveBeenCalledWith(false);
    fab.remove();
  });

  it('behavioral: rl / pt / note FABs close Builders (no parallel panels)', () => {
    const { body } = extractFabMutexListener(readMain());
    for (const className of ['rl-assistant-fab', 'pt-assistant-fab', 'note-assistant-fab']) {
      const readLaterAssistant = { setOpen: vi.fn() };
      const planTaskAssistant = { setOpen: vi.fn() };
      const noteAssistant = { setOpen: vi.fn() };
      const buildersAssistant = { setOpen: vi.fn(), close: vi.fn() };
      const fn = compileListener(body, {
        readLaterAssistant,
        planTaskAssistant,
        noteAssistant,
        buildersAssistant,
      });
      const fab = makeFab(className);
      fn({ target: fab });
      const closed =
        buildersAssistant.close.mock.calls.length > 0 ||
        buildersAssistant.setOpen.mock.calls.some((c) => c[0] === false);
      expect(closed, `${className} must close builders`).toBe(true);
      fab.remove();
    }
  });
});
