// @vitest-environment jsdom
import { act, createElement, createRef } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { ComposerInput } from '../../frontend/src/home/ui/composer-input.tsx';
import { readComposerText } from '../../frontend/src/home/commands/composer-text.ts';

let container;
let root;

afterEach(() => {
  act(() => root?.unmount());
  container?.remove();
});

function render(props = {}) {
  const ref = createRef();
  container = document.createElement('div');
  document.body.appendChild(container);
  root = createRoot(container);
  const element = (extra = {}) =>
    createElement(ComposerInput, { ref, disabled: false, placeholder: 'Ask…', ...props, ...extra });
  act(() => root.render(element()));
  return { ref, el: container.querySelector('[data-role="input"]'), rerender: (extra) => act(() => root.render(element(extra))) };
}

function caretOffset(el) {
  const sel = window.getSelection();
  const range = document.createRange();
  range.selectNodeContents(el);
  range.setEnd(sel.focusNode, sel.focusOffset);
  return range.toString().length;
}

function placeCaret(el, offset) {
  const text = el.firstChild;
  const range = document.createRange();
  range.setStart(text, offset);
  range.collapse(true);
  const sel = window.getSelection();
  sel.removeAllRanges();
  sel.addRange(range);
}

function beforeInput(el, inputType) {
  const event = new InputEvent('beforeinput', { inputType, bubbles: true, cancelable: true });
  el.dispatchEvent(event);
  return event;
}

function paste(el, data) {
  const event = new Event('paste', { bubbles: true, cancelable: true });
  event.clipboardData = { getData: (type) => data[type] ?? '' };
  el.dispatchEvent(event);
  return event;
}

describe('ComposerInput handle', () => {
  it('round-trips text including newlines and surrounding spaces', () => {
    const { ref, el } = render();
    ref.current.setText('  first\nsecond  ');
    expect(ref.current.getText()).toBe('  first\nsecond  ');
    expect(readComposerText(el)).toBe('  first\nsecond  ');
    expect(ref.current.isEmpty()).toBe(false);
  });

  it('puts the caret at the end after setText', () => {
    const { ref, el } = render();
    ref.current.setText('hello\nworld');
    expect(window.getSelection().isCollapsed).toBe(true);
    expect(caretOffset(el)).toBe('hello\nworld'.length);
  });

  it('clear empties the input and shows the placeholder', () => {
    const { ref, el } = render({ placeholder: 'Ask Lulu Spark…' });
    ref.current.setText('x');
    expect(el.getAttribute('data-empty')).toBe('false');
    ref.current.clear();
    expect(ref.current.getText()).toBe('');
    expect(ref.current.isEmpty()).toBe(true);
    expect(el.getAttribute('data-empty')).toBe('true');
    expect(el.getAttribute('data-placeholder')).toBe('Ask Lulu Spark…');
  });

  it('exposes the element and focuses it', () => {
    const { ref, el } = render();
    expect(ref.current.el).toBe(el);
    ref.current.focus();
    expect(document.activeElement).toBe(el);
  });

  it('is an editable multi-line textbox', () => {
    const { el } = render();
    expect(el.getAttribute('contenteditable')).toBe('true');
    expect(el.getAttribute('role')).toBe('textbox');
    expect(el.getAttribute('aria-multiline')).toBe('true');
  });
});

describe('ComposerInput editing', () => {
  it('turns a line break request into a newline character at the caret', () => {
    const { ref, el } = render();
    ref.current.setText('ab');
    placeCaret(el, 1);
    expect(beforeInput(el, 'insertLineBreak').defaultPrevented).toBe(true);
    expect(ref.current.getText()).toBe('a\nb');
    expect(caretOffset(el)).toBe(2);
    expect(el.querySelector('div,br,p')).toBeNull();
  });

  it('treats a paragraph request the same way', () => {
    const { ref, el } = render();
    ref.current.setText('end');
    expect(beforeInput(el, 'insertParagraph').defaultPrevented).toBe(true);
    expect(ref.current.getText()).toBe('end\n');
    expect(caretOffset(el)).toBe(4);
  });

  it('replaces a selection when inserting a newline', () => {
    const { ref, el } = render();
    ref.current.setText('hello');
    const range = document.createRange();
    range.selectNodeContents(el);
    const sel = window.getSelection();
    sel.removeAllRanges();
    sel.addRange(range);
    beforeInput(el, 'insertLineBreak');
    expect(ref.current.getText()).toBe('\n');
  });

  it('pastes plain text only and normalizes line endings', () => {
    const { ref, el } = render();
    ref.current.setText('a');
    const event = paste(el, { 'text/plain': 'x\r\ny', 'text/html': '<b>bold</b>' });
    expect(event.defaultPrevented).toBe(true);
    expect(ref.current.getText()).toBe('ax\ny');
    expect(el.querySelector('b')).toBeNull();
  });

  it('refuses rich-text formatting commands', () => {
    const { el } = render();
    for (const type of ['formatBold', 'formatItalic', 'formatUnderline']) {
      expect(beforeInput(el, type).defaultPrevented).toBe(true);
    }
  });

  it('leaves ordinary typing alone', () => {
    const { el } = render();
    expect(beforeInput(el, 'insertText').defaultPrevented).toBe(false);
  });

  it('updates the empty marker and calls onInput when the user types', () => {
    const onInput = vi.fn();
    const { el } = render({ onInput });
    expect(el.getAttribute('data-empty')).toBe('true');
    el.textContent = 'typed';
    act(() => el.dispatchEvent(new Event('input', { bubbles: true })));
    expect(el.getAttribute('data-empty')).toBe('false');
    expect(onInput).toHaveBeenCalledOnce();
    el.textContent = '';
    act(() => el.dispatchEvent(new Event('input', { bubbles: true })));
    expect(el.getAttribute('data-empty')).toBe('true');
  });

  it('drops stray line-break nodes when the input becomes empty', () => {
    const { el, ref } = render();
    el.innerHTML = '<br>';
    act(() => el.dispatchEvent(new Event('input', { bubbles: true })));
    expect(el.childNodes.length).toBe(0);
    expect(ref.current.isEmpty()).toBe(true);
  });

  it('forwards keydown and composition events', () => {
    const onKeyDown = vi.fn();
    const onCompositionStart = vi.fn();
    const onCompositionEnd = vi.fn();
    const { el } = render({ onKeyDown, onCompositionStart, onCompositionEnd });
    act(() => {
      el.dispatchEvent(new KeyboardEvent('keydown', { key: 'a', bubbles: true }));
      el.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
      el.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true }));
    });
    expect(onKeyDown).toHaveBeenCalledOnce();
    expect(onCompositionStart).toHaveBeenCalledOnce();
    expect(onCompositionEnd).toHaveBeenCalledOnce();
  });
});

describe('ComposerInput disabled', () => {
  it('is not editable while disabled and editable again afterwards', () => {
    const { el, rerender } = render({ disabled: true });
    expect(el.getAttribute('contenteditable')).toBe('false');
    expect(el.getAttribute('aria-disabled')).toBe('true');
    rerender({ disabled: false });
    expect(el.getAttribute('contenteditable')).toBe('true');
    expect(el.getAttribute('aria-disabled')).not.toBe('true');
  });

  it('keeps the typed text across a disabled period', () => {
    const { ref, rerender } = render();
    ref.current.setText('draft');
    rerender({ disabled: true });
    rerender({ disabled: false });
    expect(ref.current.getText()).toBe('draft');
  });
});

describe('readComposerText', () => {
  const read = (html) => {
    const el = document.createElement('div');
    el.innerHTML = html;
    return readComposerText(el);
  };

  it('reads text nodes, line breaks and blocks as plain text', () => {
    expect(read('a<br>b')).toBe('a\nb');
    expect(read('<div>a</div><div>b</div>')).toBe('a\nb');
    expect(read('plain')).toBe('plain');
    expect(read('<br>')).toBe('');
    expect(read('')).toBe('');
  });
});
