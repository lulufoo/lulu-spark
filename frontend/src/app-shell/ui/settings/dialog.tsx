import { useEffect, useSyncExternalStore } from 'react';
import {
  closeSettingsDialog,
  ensureWired,
  settingsOpenStore,
} from '../../commands/settings/dialog.ts';
import { SettingsDialogChrome } from './chrome.tsx';

export { closeSettingsDialog, openSettingsDialog } from '../../commands/settings/dialog.ts';

export function SettingsDialog() {
  const open = useSyncExternalStore(settingsOpenStore.subscribe, settingsOpenStore.getSnapshot);

  useEffect(() => {
    ensureWired();
  }, []);

  return (
    <div
      id="settings-dialog"
      className={open ? 'open' : undefined}
      onClick={(e) => {
        if (e.target === e.currentTarget) closeSettingsDialog();
      }}
    >
      <SettingsDialogChrome />
    </div>
  );
}
