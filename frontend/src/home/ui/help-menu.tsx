import { useState } from 'react';

export function HelpMenu({ onAbout }: { onAbout?: () => void }) {
  const [open, setOpen] = useState(false);

  function close() {
    setOpen(false);
  }

  function chooseAbout() {
    try {
      onAbout?.();
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
        aria-label="Help"
        onClick={() => setOpen((prev) => !prev)}
      >
        <span aria-hidden="true">?</span>
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
              onClick={chooseAbout}
            >
              About
            </button>
          </div>
        </>
      ) : null}
    </div>
  );
}
