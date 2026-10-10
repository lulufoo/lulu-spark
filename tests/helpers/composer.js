import { readComposerText, writeComposerText } from '../../frontend/src/home/commands/composer-text.ts';

/** The composer is a contenteditable element, not a textarea: read and write it through these. */
export function composerEl(container) {
  return container.querySelector('[data-role="input"]');
}

export function composerValue(el) {
  return readComposerText(el);
}

export function setComposerValue(el, text) {
  writeComposerText(el, text);
}

export function composerDisabled(el) {
  return el.getAttribute('contenteditable') === 'false';
}

export function composerPlaceholder(el) {
  return el.getAttribute('data-placeholder');
}
