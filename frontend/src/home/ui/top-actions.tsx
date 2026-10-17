import { HomeNavIcon } from './nav-icons.tsx';

/** Icon buttons on the right of the sidebar's top chrome row: Read Later and Settings. */
export function TopActions({
  readLaterUnread,
  onReadLater,
  onSettings,
}: {
  readLaterUnread: boolean;
  onReadLater: () => void;
  onSettings: () => void;
}) {
  return (
    <div className="home-top-actions" data-role="top-actions">
      <button
        type="button"
        className="home-top-action"
        data-home-entry="read-later"
        {...(readLaterUnread ? { 'data-home-unread': 'true' } : {})}
        aria-label="Read Later"
        title="Read Later"
        onClick={onReadLater}
      >
        <HomeNavIcon name="read-later" />
      </button>
      <button
        type="button"
        className="home-top-action"
        data-role="top-settings"
        aria-label="Settings"
        title="Settings"
        onClick={onSettings}
      >
        <HomeNavIcon name="settings" />
      </button>
    </div>
  );
}
