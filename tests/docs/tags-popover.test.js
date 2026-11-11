// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { tagAttach, tagDetach, tagUpdateValue, fetchTagsRegistry, fetchAnnotationsSummary } = vi.hoisted(() => ({
  tagAttach: vi.fn(),
  tagDetach: vi.fn(),
  tagUpdateValue: vi.fn(),
  fetchTagsRegistry: vi.fn(),
  fetchAnnotationsSummary: vi.fn(),
}));

vi.mock('../../frontend/src/host/api.ts', async (importOriginal) => ({
  ...(await importOriginal()),
  tagAttach,
  tagDetach,
  tagUpdateValue,
  fetchTagsRegistry,
  fetchAnnotationsSummary,
}));
vi.mock('../../frontend/src/notes/commands/cards.ts', () => ({ loadTitles: vi.fn() }));
vi.mock('../../frontend/src/router/index.ts', () => ({
  parseHash: vi.fn(() => ({ name: 'spark', params: {} })),
  navigateToDateList: vi.fn(),
}));

const { notifyState, state } = await import('../../frontend/src/host/state.ts');
const { closeTagsPopover, NotesTagsBar, TagsToolbarButton } = await import('../../frontend/src/notes/ui/tags-bar.tsx');

const popover = () => document.querySelector('[data-role="tags-popover"]');
const button = () => document.getElementById('btn-doc-tags');

function seedEntry(id, tags) {
  state.viewer.createSession = null;
  state.viewer.entry = { id, _id: id, raw_path: `${id}.md`, tags };
  state.index.tagsRegistry = { keys: {} };
  state.index.annotations = {};
}

function Header() {
  return createElement('div', { className: 'viewer-header-actions' }, createElement(TagsToolbarButton), createElement(NotesTagsBar));
}

let root = null;
let host = null;

beforeEach(async () => {
  tagAttach.mockReset().mockResolvedValue({ ok: true });
  tagDetach.mockReset().mockResolvedValue({ ok: true });
  tagUpdateValue.mockReset().mockResolvedValue({ ok: true });
  fetchTagsRegistry.mockReset().mockResolvedValue(null);
  fetchAnnotationsSummary.mockReset().mockResolvedValue(null);
  vi.spyOn(window, 'alert').mockImplementation(() => {});
  closeTagsPopover();
  host = document.createElement('div');
  document.body.appendChild(host);
  root = createRoot(host);
  seedEntry('doc1', [
    { key: 't1', value: 'JWT' },
    { key: 't2', value: 'Auth' },
  ]);
  await act(async () => root.render(createElement(Header)));
});

afterEach(async () => {
  closeTagsPopover();
  if (root) await act(async () => root.unmount());
  document.body.innerHTML = '';
  host = null;
  root = null;
});

async function openPopover() {
  await act(async () => {
    button().click();
  });
}

describe('tags toolbar entry', () => {
  it('renders an outlined icon button between the other header actions, hidden without an entry', async () => {
    expect(button()).not.toBeNull();
    expect(button().getAttribute('aria-label')).toBe('Tags');
    expect(button().getAttribute('aria-expanded')).toBe('false');
    expect(popover()).toBeNull();
    await act(async () => {
      state.viewer.entry = null;
      notifyState();
    });
    expect(button().style.display).toBe('none');
  });

  it('hides the entry while a create session is open', async () => {
    await act(async () => {
      state.viewer.createSession = { path: 'new.md' };
      notifyState();
    });
    expect(button().style.display).toBe('none');
  });
});

describe('tags popover shell', () => {
  it('opens under the button on click and shows existing tags and the add area', async () => {
    await openPopover();
    expect(popover()).not.toBeNull();
    expect(popover().getAttribute('role')).toBe('dialog');
    expect(popover().getAttribute('aria-label')).toBe('Tags');
    expect(popover().querySelector('.tags-popover-title').textContent).toBe('Tags');
    const chips = [...popover().querySelectorAll('.md-tag-chip')].map((c) => c.textContent);
    expect(chips).toEqual(['🏷 JWT', '🏷 Auth']);
    expect(popover().querySelector('.md-tag-entry-row input[aria-label="Tag text"]')).not.toBeNull();
    expect(button().getAttribute('aria-expanded')).toBe('true');
  });

  it('closes on a second click of the toolbar button (toggle)', async () => {
    await openPopover();
    await act(async () => {
      button().click();
    });
    expect(popover()).toBeNull();
    expect(button().getAttribute('aria-expanded')).toBe('false');
  });

  it('closes on a pointerdown outside, but not inside the popover or on the entry button', async () => {
    await openPopover();
    await act(async () => {
      popover().querySelector('.md-tag-chip').dispatchEvent(new MouseEvent('pointerdown', { bubbles: true }));
    });
    expect(popover()).not.toBeNull();
    await act(async () => {
      button().dispatchEvent(new MouseEvent('pointerdown', { bubbles: true }));
    });
    expect(popover()).not.toBeNull();
    const blank = document.createElement('div');
    document.body.appendChild(blank);
    await act(async () => {
      blank.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true }));
    });
    expect(popover()).toBeNull();
  });

  it('closes on Escape and returns focus to the toolbar button', async () => {
    await openPopover();
    await act(async () => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    });
    expect(popover()).toBeNull();
    expect(document.activeElement).toBe(button());
  });

  it('closes on window blur, the cross-origin iframe click case, without touching the iframe', async () => {
    await openPopover();
    await act(async () => {
      window.dispatchEvent(new Event('blur'));
    });
    expect(popover()).toBeNull();
  });

  it('closes when the open document changes', async () => {
    await openPopover();
    await act(async () => {
      seedEntry('doc2', []);
      notifyState();
    });
    expect(popover()).toBeNull();
  });

  it('shows the entry row with no chips when the document has no tags', async () => {
    await act(async () => {
      seedEntry('doc1', []);
      notifyState();
    });
    await openPopover();
    expect(popover().querySelectorAll('.md-tag-chip')).toHaveLength(0);
    expect(popover().querySelector('.md-tag-entry-row input')).not.toBeNull();
  });
});

describe('tags popover interactions', () => {
  const proto = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value');

  function typeInto(input, text) {
    proto.set.call(input, text);
    input.dispatchEvent(new Event('input', { bubbles: true }));
  }

  it('Enter in the entry input adds the tag, clears the input, keeps the popover open and refocuses', async () => {
    await openPopover();
    const input = popover().querySelector('.md-tag-entry-row input');
    await act(async () => {
      typeInto(input, 'Design');
    });
    await act(async () => {
      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    });
    expect(tagAttach).toHaveBeenCalledWith('doc1', { value: 'Design' });
    expect(popover()).not.toBeNull();
    expect(popover().querySelector('.md-tag-entry-row input').value).toBe('');
    expect(document.activeElement).toBe(popover().querySelector('.md-tag-entry-row input'));
  });

  it('a failed add keeps the typed text and shows the error', async () => {
    tagAttach.mockResolvedValue({ ok: false, error: 'boom' });
    await openPopover();
    const input = popover().querySelector('.md-tag-entry-row input');
    await act(async () => {
      typeInto(input, 'Nope');
    });
    await act(async () => {
      popover().querySelector('[data-action="ok"]').click();
    });
    expect(popover().querySelector('[data-role="preview"]').textContent).toContain('boom');
    expect(popover().querySelector('.md-tag-entry-row input').value).toBe('Nope');
  });

  it('registry suggestions appear while typing and a pick attaches by key', async () => {
    state.index.tagsRegistry = { keys: { k9: { value: 'JWT-Long', refs: 1 } } };
    await openPopover();
    const input = popover().querySelector('.md-tag-entry-row input');
    await act(async () => {
      typeInto(input, 'JWT');
    });
    const item = popover().querySelector('.md-tag-suggest-item');
    expect(item).not.toBeNull();
    expect(item.textContent).toBe('JWT-Long');
    await act(async () => {
      item.click();
    });
    expect(tagAttach).toHaveBeenCalledWith('doc1', { key: 'k9' });
  });

  it('clicking a chip opens a pill editor with no Save/Cancel; Enter saves the new value', async () => {
    await openPopover();
    await act(async () => {
      popover().querySelector('.md-tag-chip').click();
    });
    const editor = popover().querySelector('.md-tag-pill-editor input');
    expect(editor).not.toBeNull();
    expect(editor.value).toBe('JWT');
    expect(popover().querySelector('.md-tag-pill-editor').textContent).not.toContain('Save');
    expect(popover().querySelectorAll('.md-tag-pill-editor button')).toHaveLength(0);
    await act(async () => {
      typeInto(editor, 'JWT2');
    });
    await act(async () => {
      editor.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    });
    expect(tagUpdateValue).toHaveBeenCalledWith('t1', 'JWT2');
    expect(popover().querySelector('.md-tag-pill-editor')).toBeNull();
    expect(popover()).not.toBeNull();
  });

  it('Escape in the pill editor cancels the edit without closing the popover', async () => {
    await openPopover();
    await act(async () => {
      popover().querySelector('.md-tag-chip').click();
    });
    const editor = popover().querySelector('.md-tag-pill-editor input');
    await act(async () => {
      editor.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    });
    expect(popover().querySelector('.md-tag-pill-editor')).toBeNull();
    expect(popover()).not.toBeNull();
  });

  it('the chip remove button detaches that tag', async () => {
    await openPopover();
    await act(async () => {
      popover().querySelector('[data-action="detach"]').click();
    });
    expect(tagDetach).toHaveBeenCalledWith('doc1', 't1');
  });

  it('the Cancel button closes the popover', async () => {
    await openPopover();
    await act(async () => {
      popover().querySelector('.md-tag-entry-cancel').click();
    });
    expect(popover()).toBeNull();
    expect(document.activeElement).toBe(button());
  });
});
