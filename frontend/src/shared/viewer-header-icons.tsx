const PATHS = {
  copy:
    '<rect width="14" height="14" x="8" y="8" rx="2" ry="2" /><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" />',
  chat: '<path d="M7.9 20A9 9 0 1 0 4 16.1L2 22Z" />',
  edit:
    '<path d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z" /><path d="m15 5 4 4" />',
  comment:
    '<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" /><path d="M12 7v6" /><path d="M9 10h6" />',
} as const;

export type ViewerHeaderIconName = keyof typeof PATHS;

export function viewerHeaderIconHtml(name: ViewerHeaderIconName, filled = false) {
  const cls = filled ? 'viewer-header-icon is-filled' : 'viewer-header-icon';
  const fill = filled ? 'currentColor' : 'none';
  const stroke = filled ? 'none' : 'currentColor';
  return `<svg class="${cls}" data-viewer-icon="${name}" viewBox="0 0 24 24" aria-hidden="true" focusable="false" fill="${fill}" stroke="${stroke}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${PATHS[name]}</svg>`;
}

export function ViewerHeaderIcon({
  name,
  filled = false,
}: {
  name: ViewerHeaderIconName;
  filled?: boolean;
}) {
  return (
    <svg
      className={filled ? 'viewer-header-icon is-filled' : 'viewer-header-icon'}
      data-viewer-icon={name}
      viewBox="0 0 24 24"
      aria-hidden="true"
      focusable="false"
      fill={filled ? 'currentColor' : 'none'}
      stroke={filled ? 'none' : 'currentColor'}
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      dangerouslySetInnerHTML={{ __html: PATHS[name] }}
    />
  );
}
