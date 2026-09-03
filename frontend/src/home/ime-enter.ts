const IME_PROCESS_KEY_CODE = 229;

export type ImeKeyLike = {
  isComposing?: boolean;
  keyCode?: number;
};

export function isImeKey(event: ImeKeyLike): boolean {
  return Boolean(event.isComposing) || event.keyCode === IME_PROCESS_KEY_CODE;
}

/** Blocks the Safari/WKWebView Enter that confirms IME after compositionend. */
export function createImeEnterGuard() {
  let composing = false;
  let resetTimer: ReturnType<typeof setTimeout> | null = null;

  function clearReset() {
    if (resetTimer === null) return;
    clearTimeout(resetTimer);
    resetTimer = null;
  }

  return {
    onCompositionStart() {
      clearReset();
      composing = true;
    },
    onCompositionEnd() {
      clearReset();
      resetTimer = setTimeout(() => {
        composing = false;
        resetTimer = null;
      }, 0);
    },
    isBlocked(event: ImeKeyLike): boolean {
      return composing || isImeKey(event);
    },
    dispose() {
      clearReset();
      composing = false;
    },
  };
}
