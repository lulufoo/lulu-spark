export type OverlayDismissButtonProps = {
  id?: string;
  /** Accessible name and hover tip, e.g. "Close". */
  label?: string;
  disabled?: boolean;
  onClick?: () => void;
};

export function OverlayDismissButton({
  id,
  label,
  disabled,
  onClick,
}: OverlayDismissButtonProps) {
  return (
    <button
      type="button"
      id={id}
      className="overlay-dismiss-button"
      data-tip={label}
      aria-label={label}
      disabled={disabled}
      onClick={onClick}
    >
      ✕
    </button>
  );
}
