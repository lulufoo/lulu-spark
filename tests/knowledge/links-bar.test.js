// @vitest-environment jsdom
import { createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { notifyState, state } from '../../frontend/src/host/state.ts';
import { KbLinksBar } from '../../frontend/src/knowledge/ui/links-bar.tsx';

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchLinkTitle: vi.fn().mockResolvedValue({ title: 'Example' }),
  updateKbLinks: vi.fn(),
}));

describe('KbLinksBar', () => {
  let root;

  beforeEach(() => {
    document.body.innerHTML = '<div id="host"></div>';
    state.viewer.kbRepo = null;
    state.viewer.kbPath = null;
    state.viewer.annotation = {};
    state.index.titleFetchCache = new Map();
    root = createRoot(document.getElementById('host'));
  });

  afterEach(() => {
    flushSync(() => root.unmount());
    document.body.innerHTML = '';
  });

  it('hides the bar when a document has no links', () => {
    flushSync(() => root.render(createElement(KbLinksBar)));
    state.viewer.kbRepo = 'kb';
    state.viewer.kbPath = 'doc.md';
    state.viewer.annotation = { links: [] };
    flushSync(() => notifyState());
    expect(document.getElementById('kb-md-links-bar')?.style.display).toBe('none');
    expect(document.body.textContent).not.toMatch(/Add link/);
  });

  it('lists existing links without an Add link control', () => {
    state.viewer.kbRepo = 'kb';
    state.viewer.kbPath = 'doc.md';
    state.viewer.annotation = { links: [{ url: 'https://example.com' }] };
    flushSync(() => root.render(createElement(KbLinksBar)));
    expect(document.getElementById('kb-md-links-bar')?.style.display).toBe('flex');
    expect(document.querySelector('.kb-link-add')).toBeNull();
    expect(document.body.textContent).not.toMatch(/Add link/);
  });
});
