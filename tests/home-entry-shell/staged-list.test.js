// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { describe, it, expect, afterEach, vi } from 'vitest';
import { StagedList } from '../../frontend/src/home/ui/staged-list.tsx';
import { hydrateStaged } from '../../frontend/src/home/state/store.ts';

describe('StagedList', () => {
  let container;
  let root;

  afterEach(() => {
    act(() => {
      root?.unmount();
    });
    container?.remove();
  });

  function render(items, onOpen = vi.fn()) {
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    act(() => {
      root.render(createElement(StagedList, { items, onOpen }));
    });
  }

  it('paints staged document id and title', () => {
    render([{ id: 'F1', path: '/tmp/note.md', title: 'ReentrantLock' }]);
    const item = container.querySelector('[data-role="staged-item"]');
    expect(item).not.toBeNull();
    expect(item.getAttribute('data-staged-id')).toBe('F1');
    expect(container.querySelector('[data-role="staged-id"]').textContent).toBe('F1');
    expect(container.querySelector('[data-role="staged-title"]').textContent).toBe(
      'ReentrantLock',
    );
  });

  it('keeps F ids when hydrating binding staged', () => {
    const rows = hydrateStaged([
      { id: 'F2', path: '/tmp/b.md', title: 'Mesa' },
    ]);
    expect(rows).toEqual([{ id: 'F2', path: '/tmp/b.md', title: 'Mesa' }]);
  });
});
