import { useEffect, useSyncExternalStore } from 'react';
import { createPortal } from 'react-dom';
import {
  beginFolderCreate,
  closeFolderMenu,
  deleteEmptyFolder,
  deselectFolder,
  folderPathOf,
  openFolderMenu,
} from '../commands/folder-create.ts';
import { beginFolderRename } from '../commands/folder-rename.ts';
import { folderCreateStore } from '../state/folder-create.ts';

function targetElement(target: EventTarget | null): HTMLElement | null {
  return target instanceof HTMLElement ? target : null;
}

/** Right-click on a folder. The browser must not select the name. */
export function onFolderContextMenu(nodeKey: string, empty: boolean) {
  return (event: { preventDefault: () => void; stopPropagation: () => void; clientX: number; clientY: number }) => {
    event.preventDefault();
    event.stopPropagation();
    window.getSelection()?.removeAllRanges();
    openFolderMenu(folderPathOf(nodeKey), event.clientX, event.clientY, empty);
  };
}

/** Right-click on a file keeps the browser menu off and does not create a folder. */
export function onFileContextMenu(event: { preventDefault: () => void; stopPropagation: () => void }) {
  event.preventDefault();
  event.stopPropagation();
  window.getSelection()?.removeAllRanges();
}

/** Clicking the pane outside a row clears the listed folder. */
export function onDirectoryBackgroundClick(event: { target: EventTarget | null }) {
  const el = targetElement(event.target);
  if (!el || el.closest('.knowledge-doc-tree-node, [data-role="folder-menu"]')) return;
  closeFolderMenu();
  deselectFolder();
}

/** Right-click on the pane outside a row creates a folder at the project root. */
export function onDirectoryBackgroundMenu(event: {
  target: EventTarget | null;
  preventDefault: () => void;
  clientX: number;
  clientY: number;
}) {
  const el = targetElement(event.target);
  if (!el || el.closest('.knowledge-doc-tree-node')) return;
  event.preventDefault();
  window.getSelection()?.removeAllRanges();
  openFolderMenu('/', event.clientX, event.clientY);
}

export function FolderMenu() {
  const snap = useSyncExternalStore(folderCreateStore.subscribe, folderCreateStore.getSnapshot);
  const menu = snap.menu;

  useEffect(() => {
    if (!menu) return undefined;
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      event.preventDefault();
      closeFolderMenu();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [menu]);

  if (!menu) return null;
  return createPortal(
    <div
      data-role="folder-menu"
      className="folder-menu"
      role="menu"
      style={{ top: menu.y, left: menu.x }}
    >
      {menu.canCreate ? (
        <button
          type="button"
          role="menuitem"
          onMouseDown={(event) => {
            event.preventDefault();
            beginFolderCreate();
          }}
        >
        New folder
      </button>
      ) : null}
      {menu.canRename ? (
        <button
          type="button"
          role="menuitem"
          onMouseDown={(event) => {
            event.preventDefault();
            beginFolderRename();
          }}
        >
          Rename
        </button>
      ) : null}
      {menu.canDelete ? (
        <button
          type="button"
          role="menuitem"
          onMouseDown={(event) => {
            event.preventDefault();
            void deleteEmptyFolder();
          }}
        >
          Delete folder
        </button>
      ) : null}
    </div>,
    document.body,
  );
}
