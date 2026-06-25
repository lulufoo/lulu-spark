import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

describe('TG3 — sediment-kb list (repo-list removed)', () => {
  it('main.js defines loadSedimentKbList using fetchSedimentKbRepos', () => {
    const src = readFileSync(new URL('../frontend/js/main.js', import.meta.url), 'utf8');
    expect(src).toMatch(/async function loadSedimentKbList\b/);
    expect(src).toMatch(/fetchSedimentKbRepos\s*\(/);
  });

  it('main.js does not reference removed repo-list APIs', () => {
    const src = readFileSync(new URL('../frontend/js/main.js', import.meta.url), 'utf8');
    expect(src).not.toMatch(/fetchRepoList\b/);
    expect(src).not.toMatch(/getKbCorpusStatus\b/);
    expect(src).not.toMatch(/syncWorkbenchRepo\b/);
    expect(src).not.toMatch(/syncWorkbenchCorpus\b/);
  });

  it('loadSedimentKbList surfaces API failure as error state', () => {
    const src = readFileSync(new URL('../frontend/js/main.js', import.meta.url), 'utf8');
    expect(src).toMatch(/loadSedimentKbList[\s\S]*catch[\s\S]*#cf222e|color:\s*#cf222e/);
  });
});
