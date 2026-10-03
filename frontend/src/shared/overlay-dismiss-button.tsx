export type OverlayDismissButtonProps = {
  id?: string;
  title?: string;
  disabled?: boolean;
  onClick?: () => void;
};

export function OverlayDismissButton({
  id,
  title,
  disabled,
  onClick,
}: OverlayDismissButtonProps) {
  return (
    <button
      type="button"
      id={id}
      className="overlay-dismiss-button"
      title={title}
      aria-label={title}
      disabled={disabled}
      onClick={onClick}
    >
      ✕
    </button>
  );
}
