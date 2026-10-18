/**
 * The letter of a Cmd+<letter> shortcut, lower-cased, or null when the event is not one.
 *
 * Cmd only: Ctrl/Shift/Alt variants belong to other bindings. Key auto-repeat and IME composition
 * never count (holding ⌘O would otherwise fire once per repeat). The letter comes from `key`, so it
 * follows the keyboard layout and Caps Lock does not matter.
 */
export function cmdShortcutKey(event: KeyboardEvent): string | null {
  if (!event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) return null;
  if (event.isComposing || event.repeat) return null;
  return typeof event.key === 'string' ? event.key.toLowerCase() : null;
}
