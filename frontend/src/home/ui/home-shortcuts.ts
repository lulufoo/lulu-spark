/**
 * Home-page global shortcuts: ⌘O (new chat) and ⌘K (toggle search).
 *
 * Attached while the Home page is mounted and detached on unmount, so they do
 * nothing on other pages. macOS only: the modifier is Cmd, Ctrl is not mapped.
 * Neither combo is bound by the Tauri default menu or the webview (checked
 * against tauri 2.11 / muda 0.19), and the app registers no global shortcuts.
 */
import { cmdShortcutKey } from '../../shared/search-modal/cmd-key.ts';

export type HomeShortcutHandlers = {
  onNewChat: () => void;
  onToggleSearch: () => void;
};

let _cleanup: (() => void) | null = null;

function onKeyDownFor(handlers: HomeShortcutHandlers) {
  return (event: KeyboardEvent) => {
    const key = cmdShortcutKey(event);
    if (key === 'o') {
      event.preventDefault();
      handlers.onNewChat();
    } else if (key === 'k') {
      event.preventDefault();
      handlers.onToggleSearch();
    }
  };
}

export function detachHomeShortcuts() {
  _cleanup?.();
  _cleanup = null;
}

export function attachHomeShortcuts(handlers: HomeShortcutHandlers) {
  detachHomeShortcuts();
  const onKeyDown = onKeyDownFor(handlers);
  document.addEventListener('keydown', onKeyDown);
  _cleanup = () => document.removeEventListener('keydown', onKeyDown);
  return detachHomeShortcuts;
}
