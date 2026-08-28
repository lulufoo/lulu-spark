// @vitest-environment jsdom
import { createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { notifyState, state } from '../../frontend/src/host/state.ts';
import { NotesLinksBar } from '../../frontend/src/notes/ui/links-bar.tsx';

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchLinkTitle: vi.fn().mockResolvedValue({ title: 'Example' }),
  addNoteLink: vi.fn(),
  removeNoteLink: vi.fn(),
}));

describe('NotesLinksBar open-doc hook count', () => {
  let root;

  beforeEach(() => {
    document.body.innerHTML = '<div id="host"></div>';
    state.viewer.entry = null;
    state.viewer.createSession = null;
    state.index.titleFetchCache = new Map();
    root = createRoot(document.getElementById('host'));
  });

  afterEach(() => {
    flushSync(() => root.unmount());
    document.body.innerHTML = '';
  });

  it('does not crash when a note appears after the empty list chrome', () => {
    flushSync(() => root.render(createElement(NotesLinksBar)));
    expect(document.getElementById('md-links-bar')).toBeTruthy();

    state.viewer.entry = {
      common_path: 'inbox/notes/demo.md',
      created_at: '20260828120000',
      links: [],
    };
    expect(() => {
      flushSync(() => notifyState());
    }).not.toThrow();
    expect(document.getElementById('md-links-bar')?.style.display).toBe('flex');
  });
});
