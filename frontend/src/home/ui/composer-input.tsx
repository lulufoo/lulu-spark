import {
  forwardRef,
  useImperativeHandle,
  useLayoutEffect,
  useRef,
  type CompositionEventHandler,
  type FormEventHandler,
  type KeyboardEventHandler,
} from 'react';
import {
  attachComposerEditing,
  clearComposerText,
  composerIsEmpty,
  insertReferenceAtSelection,
  readComposerText,
  writeComposerText,
} from '../commands/composer-text.ts';
import type { ReferenceValue } from '../references/index.ts';

/** What the rest of the page may do with the composer. Nothing outside reads `.value`. */
export type ComposerInputHandle = {
  el: HTMLElement;
  focus(): void;
  getText(): string;
  /** Replace the content; the caret goes to the end. */
  setText(text: string): void;
  clear(): void;
  isEmpty(): boolean;
  /** Insert a reference chip at the caret, followed by a space. */
  insertReference(ref: ReferenceValue): void;
};

type ComposerInputProps = {
  disabled: boolean;
  placeholder: string;
  onKeyDown?: KeyboardEventHandler<HTMLDivElement>;
  onInput?: FormEventHandler<HTMLDivElement>;
  onCompositionStart?: CompositionEventHandler<HTMLDivElement>;
  onCompositionEnd?: CompositionEventHandler<HTMLDivElement>;
};

export const ComposerInput = forwardRef<ComposerInputHandle, ComposerInputProps>(
  function ComposerInput(
    { disabled, placeholder, onKeyDown, onInput, onCompositionStart, onCompositionEnd },
    ref,
  ) {
    const elRef = useRef<HTMLDivElement | null>(null);

    useImperativeHandle(
      ref,
      () => ({
        get el() {
          return elRef.current as HTMLElement;
        },
        focus() {
          elRef.current?.focus();
        },
        getText() {
          return elRef.current ? readComposerText(elRef.current) : '';
        },
        setText(text: string) {
          if (elRef.current) writeComposerText(elRef.current, text);
        },
        clear() {
          if (elRef.current) clearComposerText(elRef.current);
        },
        isEmpty() {
          return elRef.current ? composerIsEmpty(elRef.current) : true;
        },
        insertReference(value: ReferenceValue) {
          if (elRef.current) insertReferenceAtSelection(elRef.current, value);
        },
      }),
      [],
    );

    useLayoutEffect(() => {
      const el = elRef.current;
      return el ? attachComposerEditing(el) : undefined;
    }, []);

    return (
      <div
        ref={elRef}
        className="home-chat-input"
        data-role="input"
        data-placeholder={placeholder}
        role="textbox"
        aria-multiline="true"
        aria-disabled={disabled ? 'true' : undefined}
        contentEditable={!disabled}
        suppressContentEditableWarning
        spellCheck={false}
        onKeyDown={onKeyDown}
        onInput={onInput}
        onCompositionStart={onCompositionStart}
        onCompositionEnd={onCompositionEnd}
      />
    );
  },
);
