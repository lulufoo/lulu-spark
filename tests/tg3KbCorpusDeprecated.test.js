import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

describe('TG3 — KNOWLEDGE_CORPUS list uses sediment API', () => {
  it('main.js defines loadSedimentKbList using fetchSedimentKbRepos', () => {
    const src = readFileSync(new URL('../frontend/js/main.js', import.meta.url), 'utf8');
    expect(src).toMatch(/async function loadSedimentKbList\b/);
    expect(src).toMatch(/fetchSedimentKbRepos\s*\(/);
  });

  it('main.js routes KNOWLEDGE_CORPUS load through loadSedimentKbList not getKbCorpusStatus', () => {
    const src = readFileSync(new URL('../frontend/js/main.js', import.meta.url), 'utf8');
    expect(src).toMatch(
      /KNOWLEDGE_CORPUS[\s\S]*loadSedimentKbList\s*\(/,
    );
    expect(src).not.toMatch(
      /getKbCorpusStatus\s*\(\s*['"]KNOWLEDGE_CORPUS['"]\s*\)/,
    );
  });

  it('btn-kb-corpus-sync is noop for KNOWLEDGE_CORPUS', () => {
    const src = readFileSync(new URL('../frontend/js/main.js', import.meta.url), 'utf8');
    expect(src).toMatch(/btn-kb-corpus-sync[\s\S]*KNOWLEDGE_CORPUS[\s\S]*return/);
  });

  it('loadSedimentKbList surfaces API failure as error state', () => {
    const src = readFileSync(new URL('../frontend/js/main.js', import.meta.url), 'utf8');
    expect(src).toMatch(/loadSedimentKbList[\s\S]*catch[\s\S]*#cf222e|color:\s*#cf222e/);
  });
});
