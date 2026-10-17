import { useState } from 'react';
import { HomeNavIcon } from './nav-icons.tsx';

/**
 * The `?` at the right of the account row. Its menu holds About today; more help entries go in
 * the same list. Settings stays in the top bar; device pairing is a page inside Settings.
 */
export function HelpMenu({ onAbout }: { onAbout?: () => void }) {
  const [open, setOpen] = useState(false);

  function close() {
    setOpen(false);
  }

  function choose(action?: () => void) {
    try {
      action?.();
    } catch {
      /* keep the menu clickable */
    }
    close();
  }

  return (
    <div className="home-help-menu" data-role="help-menu">
      <button
        type="button"
        className="home-help-menu-trigger"
        data-role="help-menu-trigger"
        aria-haspopup="menu"
        aria-expanded={open ? 'true' : 'false'}
        aria-label="About"
        data-tip="About"
        onClick={() => setOpen((prev) => !prev)}
      >
        <HomeNavIcon name="help" />
      </button>
      {open ? (
        <>
          <div
            className="home-help-menu-backdrop"
            data-role="close-help-menu"
            onClick={close}
          />
          <div className="home-help-menu-panel" role="menu">
            <button
              type="button"
              role="menuitem"
              data-role="help-menu-about"
              onClick={() => choose(onAbout)}
            >
              About
            </button>
          </div>
        </>
      ) : null}
    </div>
  );
}
