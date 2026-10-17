// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { repoRoot } from '../helpers/read-frontend-js.js';

vi.mock('../../frontend/src/host/api.ts', () => ({ invoke: vi.fn(), fetchFileContent: vi.fn() }));

import { NotesDocList } from '../../frontend/src/notes/ui/cards.tsx';

function readRel(rel) {
  return readFileSync(`${repoRoot}/${rel}`, 'utf8');
}

describe('notes list empty states', () => {
  let container;
  let root;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  function render(props) {
    act(() => {
      root.render(createElement(NotesDocList, { entries: [], date: '', ...props }));
    });
  }

  it('paints nothing when empty without a message (the toolbar heading already says it)', () => {
    render({ empty: true });
    expect(container.querySelector('.empty-filter-msg')).toBeNull();
    expect(container.textContent).toBe('');
    expect(container.textContent).not.toMatch(/match the current filters/);
  });

  it('still paints the directory hint for an empty folder', () => {
    render({ empty: true, emptyMessage: 'No documents directly in this folder. Open it in the tree.' });
    expect(container.querySelector('.empty-filter-msg')?.textContent).toBe(
      'No documents directly in this folder. Open it in the tree.',
    );
  });

  it('only the empty folder asks the list for a message; the no-match heading stays on the toolbar', () => {
    const page = readRel('frontend/src/notes/page.tsx');
    expect(page).toMatch(/empty=\{emptyFolder\}/);
    expect(page).not.toMatch(/empty=\{emptyFilter/);
    expect(page).toMatch(/'No matching items'/);
    expect(readRel('frontend/src/notes/ui/cards.tsx')).not.toMatch(/match the current filters/);
  });
});
