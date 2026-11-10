import { renameSelectedFile } from './file-rename.ts';
import { renameSelectedFolder } from './folder-rename.ts';

function onDirectoryRenameEnter(event: KeyboardEvent) {
  if (event.defaultPrevented) return;
  if (event.key !== 'Enter' || event.repeat) return;
  if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) return;
  if (event.isComposing) return;
  if (renameSelectedFile(event.target) || renameSelectedFolder(event.target)) {
    event.preventDefault();
  }
}

/** While the directory tree is up: Enter edits the open file, or the listed folder. */
export function attachDirectoryRenameEnter() {
  document.addEventListener('keydown', onDirectoryRenameEnter, true);
  return () => document.removeEventListener('keydown', onDirectoryRenameEnter, true);
}
