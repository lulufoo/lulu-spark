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

  it('renders desktop shortcuts without page heading labels', () => {
    mountHomeHub(container, { navigate });

    expect(container.querySelector('.home-hub-title')).toBeNull();
    expect(container.querySelector('.home-hub-subtitle')).toBeNull();
    expect(container.querySelector('.home-desktop')).not.toBeNull();
    expect(container.querySelector('.home-desktop-wallpaper')).not.toBeNull();

    const shortcuts = container.querySelectorAll('.home-desktop-shortcut');
    expect(shortcuts).toHaveLength(3);

    const workbenchEntry = container.querySelector('[data-home-entry="workbench"]');
    const readLaterEntry = container.querySelector('[data-home-entry="read-later"]');
    const corpusEntry = container.querySelector('[data-home-entry="corpus"]');
    expect(workbenchEntry).not.toBeNull();
    expect(readLaterEntry).not.toBeNull();
    expect(corpusEntry).not.toBeNull();
    expect(workbenchEntry.textContent).toMatch(/workbench|归档/i);
    expect(readLaterEntry.textContent).toMatch(/read later|待读/i);
    expect(corpusEntry.textContent).toMatch(/沉淀|知识库/i);
  });

  it('navigates to #/workbench when workbench entry is clicked', () => {
    mountHomeHub(container, { navigate });

    container.querySelector('[data-home-entry="workbench"]').click();
    expect(navigate).toHaveBeenCalledWith('#/workbench');
  });

  it('navigates to #/corpus when corpus entry is clicked', () => {
    mountHomeHub(container, { navigate });

    container.querySelector('[data-home-entry="corpus"]').click();
    expect(navigate).toHaveBeenCalledWith('#/corpus');
  });

  it('opens read-later dialog when read-later entry is clicked', () => {
    const openReadLater = vi.fn();
    mountHomeHub(container, { navigate, openReadLater });

    container.querySelector('[data-home-entry="read-later"]').click();
    expect(openReadLater).toHaveBeenCalledTimes(1);
    expect(navigate).not.toHaveBeenCalledWith('#/read-later');
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

  it('main.js swaps left header title for back link off home', () => {
    expect(mainJs).toMatch(/btn-nav-home-title/);
    expect(mainJs).toMatch(/homeTitle\) homeTitle\.hidden = !onHome/);
    expect(mainJs).toMatch(/homeNav\) homeNav\.hidden = onHome/);
  });
});
