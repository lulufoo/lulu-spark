// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { readFileSync } from 'node:fs';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const openDoc = vi.hoisted(() => vi.fn());
vi.mock('../../frontend/src/home/commands/open-doc-link.ts', () => ({ openDocById: openDoc }));

import { UserMessageText } from '../../frontend/src/home/ui/user-message-text.tsx';

const NOTE_ID = 'f6692dc5d5242eec5206b6400e704bd6';
const GUIDE_ID = 'knowledge-0123456789abcdef0123456789abcdef';
const NOTE = `[Default：Agent Mock](doc:${NOTE_ID})`;
const GUIDE = `[Knowledge：Guide](doc:${GUIDE_ID})`;

let container;
let root;

beforeEach(() => {
  openDoc.mockClear();
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

  it('shows document references of two workspaces as chips between the text', () => {
    const el = render(`read ${NOTE} then ${GUIDE} now`);
    const chips = [...el.querySelectorAll('.home-ref-chip')];
    expect(chips.map((c) => c.textContent)).toEqual(['Default：Agent Mock', 'Knowledge：Guide']);
    expect(el.textContent).toBe('read Default：Agent Mock then Knowledge：Guide now');
    expect(chips[0].querySelector('.home-ref-chip-label')?.textContent).toBe('Default：Agent Mock');
    expect(chips[0].getAttribute('data-ref-kind')).toBe('doc');
    expect(chips[0].getAttribute('data-ref-id')).toBe(NOTE_ID);
  });

  it('leaves invalid references as the original text', () => {
    const el = render('see [x](doc:abc) and [y](https://a.b)');
    expect(el.querySelector('.home-ref-chip')).toBeNull();
    expect(el.textContent).toBe('see [x](doc:abc) and [y](https://a.b)');
  });

  it('never turns message text into markup', () => {
    const el = render(`<b>x</b> [T](doc:${NOTE_ID}) <img src=x>`);
    expect(el.querySelector('b')).toBeNull();
    expect(el.querySelector('img')).toBeNull();
    expect(el.textContent).toContain('<b>x</b>');
  });

  it('opens the target when a chip is clicked', () => {
    const el = render(`${NOTE} and ${GUIDE}`);
    const [note, knowledge] = el.querySelectorAll('.home-ref-chip');
    act(() => note.click());
    act(() => knowledge.click());
    expect(openDoc).toHaveBeenCalledWith(NOTE_ID);
    expect(openDoc).toHaveBeenCalledWith(GUIDE_ID);
  });

  it('is operable from the keyboard', () => {
    const el = render(NOTE);
    const chip = el.querySelector('.home-ref-chip');
    expect(chip.getAttribute('role')).toBe('link');
    expect(chip.getAttribute('tabindex')).toBe('0');
    act(() => {
      chip.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    });
    expect(openDoc).toHaveBeenCalledWith(NOTE_ID);
  });
});

describe('home chat wiring', () => {
  const page = readFileSync('frontend/src/home/page.tsx', 'utf8');

  it('renders user messages through UserMessageText and still copies the raw text', () => {
    expect(page).toMatch(/<UserMessageText text=\{m\.text\} \/>/);
    expect(page).toMatch(/copyMessageText\(m\.text\)/);
  });
});
