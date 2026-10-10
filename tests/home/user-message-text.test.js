// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { readFileSync } from 'node:fs';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const openNote = vi.hoisted(() => vi.fn());
const openKnowledge = vi.hoisted(() => vi.fn());
vi.mock('../../frontend/src/home/commands/open-note-link.ts', () => ({ openNoteById: openNote }));
vi.mock('../../frontend/src/home/commands/open-knowledge-link.ts', () => ({
  openKnowledgeById: openKnowledge,
}));

import { UserMessageText } from '../../frontend/src/home/ui/user-message-text.tsx';

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

function render(text) {
  container = document.createElement('div');
  document.body.appendChild(container);
  root = createRoot(container);
  act(() => root.render(createElement(UserMessageText, { text })));
  return container;
}

describe('UserMessageText', () => {
  it('keeps a plain message as plain text', () => {
    const el = render('hello\nworld  spaced');
    expect(el.textContent).toBe('hello\nworld  spaced');
    expect(el.querySelector('.home-ref-chip')).toBeNull();
  });

  it('shows note and knowledge references as chips between the text', () => {
    const el = render(`read ${NOTE} then [Guide](knowledge:ab12cd) now`);
    const chips = [...el.querySelectorAll('.home-ref-chip')];
    expect(chips.map((c) => c.textContent)).toEqual(['note: Agent Mock', 'knowledge: Guide']);
    expect(el.textContent).toBe('read note: Agent Mock then knowledge: Guide now');
    expect(chips[0].getAttribute('data-ref-kind')).toBe('note');
    expect(chips[0].getAttribute('data-ref-id')).toBe(NOTE_ID);
  });

  it('leaves invalid references as the original text', () => {
    const el = render('see [x](note:abc) and [y](https://a.b)');
    expect(el.querySelector('.home-ref-chip')).toBeNull();
    expect(el.textContent).toBe('see [x](note:abc) and [y](https://a.b)');
  });

  it('never turns message text into markup', () => {
    const el = render(`<b>x</b> [T](note:${NOTE_ID}) <img src=x>`);
    expect(el.querySelector('b')).toBeNull();
    expect(el.querySelector('img')).toBeNull();
    expect(el.textContent).toContain('<b>x</b>');
  });

  it('opens the target when a chip is clicked', () => {
    const el = render(`${NOTE} and [Guide](knowledge:ab12cd)`);
    const [note, knowledge] = el.querySelectorAll('.home-ref-chip');
    act(() => note.click());
    act(() => knowledge.click());
    expect(openNote).toHaveBeenCalledWith(NOTE_ID);
    expect(openKnowledge).toHaveBeenCalledWith('ab12cd');
  });

  it('is operable from the keyboard', () => {
    const el = render(NOTE);
    const chip = el.querySelector('.home-ref-chip');
    expect(chip.getAttribute('role')).toBe('link');
    expect(chip.getAttribute('tabindex')).toBe('0');
    act(() => {
      chip.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    });
    expect(openNote).toHaveBeenCalledWith(NOTE_ID);
  });
});

describe('home chat wiring', () => {
  const page = readFileSync('frontend/src/home/page.tsx', 'utf8');

  it('renders user messages through UserMessageText and still copies the raw text', () => {
    expect(page).toMatch(/<UserMessageText text=\{m\.text\} \/>/);
    expect(page).toMatch(/copyMessageText\(m\.text\)/);
  });
});
