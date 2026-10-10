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

function textarea() {
  return document.createElement('textarea');
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
    const input = textarea();
    document.body.appendChild(input);
    requestComposerDraft('[A](note:x)');
    applyComposerDraft(input);
    expect(input.value).toBe('[A](note:x)');
    expect(input.selectionStart).toBe(input.value.length);
    expect(input.selectionEnd).toBe(input.value.length);
    input.remove();
  });

  it('leaves what the user typed alone when there is no draft', () => {
    const input = textarea();
    input.value = 'typing';
    applyComposerDraft(input);
    expect(input.value).toBe('typing');
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
