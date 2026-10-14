import { useState } from 'react';
import { HomeNavIcon } from './nav-icons.tsx';

export function HelpMenu({
  onSettings,
  onDevices,
  onAbout,
}: {
  onSettings?: () => void;
  onDevices?: () => void;
  onAbout?: () => void;
}) {
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
        aria-label="Settings"
        title="Settings"
        onClick={() => setOpen((prev) => !prev)}
      >
        <HomeNavIcon name="settings" />
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
              id="btn-settings"
              data-role="help-menu-settings"
              onClick={() => choose(onSettings)}
            >
              Settings
            </button>
            <button
              type="button"
              role="menuitem"
              id="btn-bind"
              data-role="help-menu-devices"
              onClick={() => choose(onDevices)}
            >
              Devices
            </button>
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
