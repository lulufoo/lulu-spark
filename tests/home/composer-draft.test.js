// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { afterEach, describe, expect, it } from 'vitest';

import {
  applyComposerDraft,
  cancelComposerDraft,
  consumeComposerDraft,
  requestComposerDraft,
} from '../../frontend/src/home/commands/composer-draft.ts';
import { consumeComposerFocus } from '../../frontend/src/home/commands/composer-focus.ts';
import { readComposerText, writeComposerText } from '../../frontend/src/home/commands/composer-text.ts';

function composer() {
  const el = document.createElement('div');
  el.setAttribute('contenteditable', 'true');
  el.setAttribute('data-role', 'input');
  document.body.appendChild(el);
  return {
    el,
    setText: (text) => writeComposerText(el, text),
    getText: () => readComposerText(el),
  };
}

function caretOffset(el) {
  const sel = window.getSelection();
  const range = document.createRange();
  range.selectNodeContents(el);
  range.setEnd(sel.focusNode, sel.focusOffset);
  return range.toString().length;
}

describe('composer draft', () => {
  afterEach(() => {
    cancelComposerDraft();
  });

  it('a request asks for focus and holds the text until consumed', () => {
    requestComposerDraft('[A](note:x)');
    expect(consumeComposerFocus()).toBe(true);
    expect(consumeComposerDraft()).toBe('[A](note:x)');
  });

  it('is one-shot', () => {
    requestComposerDraft('hello');
    expect(consumeComposerDraft()).toBe('hello');
    expect(consumeComposerDraft()).toBeNull();
  });

  it('applies the draft with the caret at the end', () => {
    const input = composer();
    requestComposerDraft('[A](note:x)');
    applyComposerDraft(input);
    expect(input.getText()).toBe('[A](note:x)');
    expect(caretOffset(input.el)).toBe('[A](note:x)'.length);
    input.el.remove();
  });

  it('leaves what the user typed alone when there is no draft', () => {
    const input = composer();
    input.setText('typing');
    applyComposerDraft(input);
    expect(input.getText()).toBe('typing');
    input.el.remove();
  });

  it('cancel drops both the draft and the pending focus', () => {
    requestComposerDraft('hello');
    cancelComposerDraft();
    expect(consumeComposerDraft()).toBeNull();
    expect(consumeComposerFocus()).toBe(false);
  });

  it('the home page applies the draft in the same effect that takes the focus', () => {
    const page = readFileSync('frontend/src/home/page.tsx', 'utf8');
    expect(page).toMatch(
      /if \(!consumeComposerFocus\(\)\) return;\s*inputRef\.current\?\.focus\(\);\s*applyComposerDraft\(inputRef\.current\);/,
    );
  });
});
