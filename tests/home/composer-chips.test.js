// @vitest-environment jsdom
import { act, createElement, createRef } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const openNote = vi.hoisted(() => vi.fn());
const openKnowledge = vi.hoisted(() => vi.fn());
vi.mock('../../frontend/src/home/commands/open-note-link.ts', () => ({ openNoteById: openNote }));
vi.mock('../../frontend/src/home/commands/open-knowledge-link.ts', () => ({
  openKnowledgeById: openKnowledge,
}));

import { ComposerInput } from '../../frontend/src/home/ui/composer-input.tsx';
import { registerReferenceKind } from '../../frontend/src/home/references/index.ts';

const NOTE_ID = 'f6692dc5d5242eec5206b6400e704bd6';
const NOTE = `[Agent Mock](note:${NOTE_ID})`;

let container;
let root;

beforeEach(() => {
  openNote.mockClear();
  openKnowledge.mockClear();
});

afterEach(() => {
  act(() => root?.unmount());
  container?.remove();
});

function render(props = {}) {
  const ref = createRef();
  container = document.createElement('div');
  document.body.appendChild(container);
  root = createRoot(container);
  act(() =>
    root.render(createElement(ComposerInput, { ref, disabled: false, placeholder: 'Ask…', ...props })),
  );
  return { ref, el: container.querySelector('[data-role="input"]') };
}

function caretAt(node, offset) {
  const range = document.createRange();
  range.setStart(node, offset);
  range.collapse(true);
  const sel = window.getSelection();
  sel.removeAllRanges();
  sel.addRange(range);
}

function selectAll(el) {
  const range = document.createRange();
  range.selectNodeContents(el);
  const sel = window.getSelection();
  sel.removeAllRanges();
  sel.addRange(range);
}

function beforeInput(el, inputType) {
  const event = new InputEvent('beforeinput', { inputType, bubbles: true, cancelable: true });
  el.dispatchEvent(event);
  return event;
}

function clipboardEvent(el, type, data = {}) {
  const event = new Event(type, { bubbles: true, cancelable: true });
  const written = {};
  event.clipboardData = {
    getData: (kind) => data[kind] ?? '',
    setData: (kind, value) => {
      written[kind] = value;
    },
  };
  el.dispatchEvent(event);
  return { event, written };
}

describe('chips in the composer', () => {
  it('shows a note reference as a chip and keeps the markdown underneath', () => {
    const { ref, el } = render();
    ref.current.setText(`read ${NOTE} now`);
    const chips = el.querySelectorAll('.home-ref-chip');
    expect(chips).toHaveLength(1);
    const chip = chips[0];
    expect(chip.getAttribute('contenteditable')).toBe('false');
    expect(chip.getAttribute('data-ref-kind')).toBe('note');
    expect(chip.getAttribute('data-ref-id')).toBe(NOTE_ID);
    expect(chip.textContent).toBe('note: Agent Mock');
    expect(chip.querySelector('.home-ref-chip-label')?.textContent).toBe('note: Agent Mock');
    expect(el.textContent).not.toContain('(note:');
    expect(ref.current.getText()).toBe(`read ${NOTE} now`);
  });

  it('shows a knowledge reference with its own label', () => {
    const { ref, el } = render();
    ref.current.setText('[Guide](knowledge:ab12cd)');
    expect(el.querySelector('.home-ref-chip').textContent).toBe('knowledge: Guide');
    expect(ref.current.getText()).toBe('[Guide](knowledge:ab12cd)');
  });

  it('keeps an invalid reference as plain text', () => {
    const { ref, el } = render();
    ref.current.setText('[x](note:abc) and [y](https://a.b)');
    expect(el.querySelector('.home-ref-chip')).toBeNull();
    expect(ref.current.getText()).toBe('[x](note:abc) and [y](https://a.b)');
  });

  it('counts a lone chip as content', () => {
    const { ref, el } = render();
    ref.current.setText(NOTE);
    expect(ref.current.isEmpty()).toBe(false);
    expect(el.getAttribute('data-empty')).toBe('false');
  });

  it('inserts a reference at the caret followed by a space', () => {
    const { ref, el } = render();
    ref.current.setText('see ');
    ref.current.insertReference({ kind: 'note', id: NOTE_ID, title: 'Agent Mock' });
    expect(el.querySelectorAll('.home-ref-chip')).toHaveLength(1);
    expect(ref.current.getText()).toBe(`see ${NOTE} `);
  });
});

describe('deleting a chip', () => {
  it('Backspace right after a chip removes the whole chip', () => {
    const onInput = vi.fn();
    const { ref, el } = render({ onInput });
    ref.current.setText(`${NOTE}x`);
    caretAt(el.lastChild, 0);
    expect(beforeInput(el, 'deleteContentBackward').defaultPrevented).toBe(true);
    expect(el.querySelector('.home-ref-chip')).toBeNull();
    expect(ref.current.getText()).toBe('x');
    expect(onInput).toHaveBeenCalled();
  });

  it('Backspace after a chip at the end of the line removes it too', () => {
    const { ref, el } = render();
    ref.current.setText(NOTE);
    caretAt(el, el.childNodes.length);
    expect(beforeInput(el, 'deleteContentBackward').defaultPrevented).toBe(true);
    expect(ref.current.getText()).toBe('');
    expect(el.getAttribute('data-empty')).toBe('true');
  });

  it('Delete right before a chip removes the whole chip', () => {
    const { ref, el } = render();
    ref.current.setText(`x${NOTE}`);
    caretAt(el.firstChild, 1);
    expect(beforeInput(el, 'deleteContentForward').defaultPrevented).toBe(true);
    expect(ref.current.getText()).toBe('x');
  });

  it('leaves ordinary deletion to the browser', () => {
    const { ref, el } = render();
    ref.current.setText(`${NOTE}xy`);
    caretAt(el.lastChild, 1);
    expect(beforeInput(el, 'deleteContentBackward').defaultPrevented).toBe(false);
    expect(el.querySelector('.home-ref-chip')).not.toBeNull();
  });

  it('leaves a ranged selection to the browser', () => {
    const { ref, el } = render();
    ref.current.setText(`${NOTE}xy`);
    const range = document.createRange();
    range.setStart(el.lastChild, 0);
    range.setEnd(el.lastChild, 1);
    const sel = window.getSelection();
    sel.removeAllRanges();
    sel.addRange(range);
    expect(beforeInput(el, 'deleteContentBackward').defaultPrevented).toBe(false);
  });
});

describe('Command-Backspace line delete', () => {
  it('deletes a trailing chip and the rest of the line', () => {
    const onInput = vi.fn();
    const { ref, el } = render({ onInput });
    ref.current.setText(`${NOTE} `);
    expect(beforeInput(el, 'deleteSoftLineBackward').defaultPrevented).toBe(true);
    expect(el.querySelector('.home-ref-chip')).toBeNull();
    expect(ref.current.getText()).toBe('');
    expect(el.getAttribute('data-empty')).toBe('true');
    expect(window.getSelection().isCollapsed).toBe(true);
    expect(onInput).toHaveBeenCalled();
  });

  it('treats deleteHardLineBackward the same way', () => {
    const { ref, el } = render();
    ref.current.setText(`${NOTE} `);
    expect(beforeInput(el, 'deleteHardLineBackward').defaultPrevented).toBe(true);
    expect(ref.current.getText()).toBe('');
    expect(el.querySelector('.home-ref-chip')).toBeNull();
  });

  it('keeps the previous line', () => {
    const { ref, el } = render();
    ref.current.setText(`keep\n${NOTE} `);
    expect(beforeInput(el, 'deleteSoftLineBackward').defaultPrevented).toBe(true);
    expect(ref.current.getText()).toBe('keep\n');
    expect(el.querySelector('.home-ref-chip')).toBeNull();
  });

  it('does not delete when the caret is already at the line start', () => {
    const { ref, el } = render();
    ref.current.setText(`${NOTE} `);
    caretAt(el, 0);
    expect(beforeInput(el, 'deleteSoftLineBackward').defaultPrevented).toBe(true);
    expect(ref.current.getText()).toBe(`${NOTE} `);
    expect(el.querySelector('.home-ref-chip')).not.toBeNull();
  });

  it('deletes only the selection when one exists', () => {
    const { ref, el } = render();
    ref.current.setText(`${NOTE}xy`);
    const range = document.createRange();
    range.setStart(el.lastChild, 0);
    range.setEnd(el.lastChild, 1);
    const sel = window.getSelection();
    sel.removeAllRanges();
    sel.addRange(range);
    expect(beforeInput(el, 'deleteSoftLineBackward').defaultPrevented).toBe(true);
    expect(ref.current.getText()).toBe(`${NOTE}y`);
    expect(el.querySelector('.home-ref-chip')).not.toBeNull();
  });
});

describe('clicking a chip', () => {
  it('opens a note through the note opener', () => {
    const { ref, el } = render();
    ref.current.setText(NOTE);
    act(() => el.querySelector('.home-ref-chip').click());
    expect(openNote).toHaveBeenCalledWith(NOTE_ID);
    expect(openKnowledge).not.toHaveBeenCalled();
  });

  it('opens a knowledge document through the knowledge opener', () => {
    const { ref, el } = render();
    ref.current.setText('[Guide](knowledge:ab12cd)');
    act(() => el.querySelector('.home-ref-chip').click());
    expect(openKnowledge).toHaveBeenCalledWith('ab12cd');
  });

  it('opens any registered kind through its own opener', () => {
    const open = vi.fn();
    registerReferenceKind({ kind: 'doc', scheme: 'doc', label: 'doc', idPattern: /^[a-z]+$/, open });
    const { ref, el } = render();
    ref.current.setText('[T](doc:abc)');
    act(() => el.querySelector('.home-ref-chip').click());
    expect(open).toHaveBeenCalledWith('abc');
  });

  it('does nothing when the click is on ordinary text', () => {
    const { ref, el } = render();
    ref.current.setText(`hello ${NOTE}`);
    act(() => el.firstChild.parentElement.click());
    expect(openNote).not.toHaveBeenCalled();
  });
});

describe('clipboard', () => {
  it('copies the selection as markdown, not as the chip label', () => {
    const { ref, el } = render();
    ref.current.setText(`a ${NOTE} b`);
    selectAll(el);
    const { event, written } = clipboardEvent(el, 'copy');
    expect(event.defaultPrevented).toBe(true);
    expect(written['text/plain']).toBe(`a ${NOTE} b`);
  });

  it('copies only the selected part', () => {
    const { ref, el } = render();
    ref.current.setText(`a ${NOTE} b`);
    const range = document.createRange();
    range.setStart(el.firstChild, 2);
    range.setEnd(el.lastChild, 0);
    const sel = window.getSelection();
    sel.removeAllRanges();
    sel.addRange(range);
    expect(clipboardEvent(el, 'copy').written['text/plain']).toBe(NOTE);
  });

  it('cut copies as markdown and removes the selection', () => {
    const onInput = vi.fn();
    const { ref, el } = render({ onInput });
    ref.current.setText(`a ${NOTE}`);
    selectAll(el);
    const { written } = clipboardEvent(el, 'cut');
    expect(written['text/plain']).toBe(`a ${NOTE}`);
    expect(ref.current.getText()).toBe('');
    expect(onInput).toHaveBeenCalled();
  });

  it('turns pasted markdown references back into chips', () => {
    const { ref, el } = render();
    ref.current.setText('x ');
    clipboardEvent(el, 'paste', { 'text/plain': `${NOTE} tail` });
    expect(el.querySelectorAll('.home-ref-chip')).toHaveLength(1);
    expect(ref.current.getText()).toBe(`x ${NOTE} tail`);
  });
});
