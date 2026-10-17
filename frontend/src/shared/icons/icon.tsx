import { getIcon } from './index.ts';

type Props = {
  id: string;
  /** Rendered width and height in px; the drawing itself is a 24x24 outline. */
  size?: number;
  className?: string;
};

/** A built-in icon in the current text colour. Renders nothing for an unknown id. */
export function Icon({ id, size = 16, className }: Props) {
  const icon = getIcon(id);
  if (!icon) return null;
  return (
    <svg
      className={className ? `ui-icon ${className}` : 'ui-icon'}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      data-icon-id={icon.id}
      dangerouslySetInnerHTML={{ __html: icon.body }}
    />
  );
}
