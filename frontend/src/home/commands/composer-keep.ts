/** The composer's unsent text. The home page is unmounted whenever the user leaves #/home,
 * so the text is kept here and put back when the composer mounts again. */
let keptText = '';

export function saveComposerText(text: string): void {
  keptText = text;
}

export function loadComposerText(): string {
  return keptText;
}

/** Tests only. */
export function resetComposerKeep(): void {
  keptText = '';
}
