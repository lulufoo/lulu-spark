import { openFilePopup as openFilePopupCommand } from './commands/popup.ts';
import { ensureFilePopupHost } from './ui/popup.tsx';
import type { OpenFilePopupInput } from './commands/popup.ts';

export type { OpenFilePopupInput };
export { closeFilePopup } from './commands/popup.ts';
export { FilePopup } from './ui/popup.tsx';

/** Caller gives an absolute path and optional title. Close does not change the route. */
export function openFilePopup(input: OpenFilePopupInput) {
  ensureFilePopupHost();
  void openFilePopupCommand(input);
}
