/** Paint-only switch. The caller owns the collapsed flag, the nav id, and the toggle. */
export function SidebarToggle({
  collapsed,
  controlsId,
  onToggle,
}: {
  collapsed: boolean;
  controlsId: string;
  onToggle: () => void;
}) {
  const label = collapsed ? 'Show sidebar' : 'Hide sidebar';
  return (
    <button
      type="button"
      className="sidebar-toggle"
      data-sidebar-toggle=""
      data-tip={label}
      aria-label={label}
      aria-expanded={!collapsed}
      aria-controls={controlsId}
      onClick={onToggle}
    >
      <svg className="sidebar-toggle-icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
        <rect x="3" y="4" width="18" height="16" rx="3" />
        <path d="M9 4v16" />
      </svg>
    </button>
  );
}
