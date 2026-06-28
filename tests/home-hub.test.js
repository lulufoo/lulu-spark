// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mountHomeHub } from '../frontend/js/components/home-hub.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const mainJs = readFileSync(join(fixtureRoot, 'frontend/js/main.js'), 'utf8');

describe('mountHomeHub', () => {
  let container;
  let navigate;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    navigate = vi.fn();
  });

  afterEach(() => {
    container.remove();
  });

  it('renders dual-entry hub with workbench and corpus pick links', () => {
    mountHomeHub(container, { navigate });

    const workbenchEntry = container.querySelector('[data-home-entry="workbench"]');
    const corpusEntry = container.querySelector('[data-home-entry="corpus-pick"]');
    expect(workbenchEntry).not.toBeNull();
    expect(corpusEntry).not.toBeNull();
    expect(workbenchEntry.textContent).toMatch(/workbench|归档/i);
    expect(corpusEntry.textContent).toMatch(/沉淀|知识库|选库/i);
  });

  it('navigates to #/workbench when workbench entry is clicked', () => {
    mountHomeHub(container, { navigate });

    container.querySelector('[data-home-entry="workbench"]').click();
    expect(navigate).toHaveBeenCalledWith('#/workbench');
  });

  it('navigates to #/corpus/pick when corpus pick entry is clicked', () => {
    mountHomeHub(container, { navigate });

    container.querySelector('[data-home-entry="corpus-pick"]').click();
    expect(navigate).toHaveBeenCalledWith('#/corpus/pick');
  });

  it('returns cleanup that clears container', () => {
    const cleanup = mountHomeHub(container, { navigate });
    expect(typeof cleanup).toBe('function');
    cleanup();
    expect(container.innerHTML).toBe('');
  });
});

describe('home hub shell integration', () => {
  it('main.js mounts HomeHub on home route with Phase2 default landing', () => {
    expect(mainJs).toMatch(/mountHomeHub/);
    expect(mainJs).not.toMatch(/home:\s*redirectToWorkbench/);
    expect(mainJs).toMatch(/fallback:\s*['"]#\/home['"]/);
  });
});
