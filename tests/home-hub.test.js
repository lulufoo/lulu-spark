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
    expect(shortcuts).toHaveLength(4);

    const workbenchEntry = container.querySelector('[data-home-entry="workbench"]');
    const readLaterEntry = container.querySelector('[data-home-entry="read-later"]');
    const corpusEntry = container.querySelector('[data-home-entry="corpus"]');
    const todoTasksEntry = container.querySelector('[data-home-entry="todo-tasks"]');
    expect(workbenchEntry).not.toBeNull();
    expect(readLaterEntry).not.toBeNull();
    expect(corpusEntry).not.toBeNull();
    expect(todoTasksEntry).not.toBeNull();
    expect(workbenchEntry.textContent).toMatch(/Notes/);
    expect(readLaterEntry.textContent).toMatch(/Read Later/);
    expect(corpusEntry.textContent).toMatch(/Knowledge/);
    expect(todoTasksEntry.textContent).toMatch(/Todos/);
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

  it('navigates to #/todo-tasks when todo-tasks entry is clicked', () => {
    mountHomeHub(container, { navigate });

    container.querySelector('[data-home-entry="todo-tasks"]').click();
    expect(navigate).toHaveBeenCalledWith('#/todo-tasks');
  });

  it('does not throw when todo-tasks entry is clicked without navigate', () => {
    mountHomeHub(container, {});

    expect(() => {
      container.querySelector('[data-home-entry="todo-tasks"]').click();
    }).not.toThrow();
  });

  it('returns cleanup that clears container', () => {
    const cleanup = mountHomeHub(container, { navigate });
    expect(typeof cleanup).toBe('function');
    cleanup();
    expect(container.innerHTML).toBe('');
  });

  it('does not leak listeners after repeated mount and unmount', () => {
    const addSpy = vi.spyOn(HTMLElement.prototype, 'addEventListener');
    const removeSpy = vi.spyOn(HTMLElement.prototype, 'removeEventListener');

    const cleanup1 = mountHomeHub(container, { navigate });
    cleanup1();
    const cleanup2 = mountHomeHub(container, { navigate });
    cleanup2();

    const clickAdds = addSpy.mock.calls.filter(([type]) => type === 'click').length;
    const clickRemoves = removeSpy.mock.calls.filter(([type]) => type === 'click').length;
    expect(clickAdds).toBe(clickRemoves);

    addSpy.mockRestore();
    removeSpy.mockRestore();
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
