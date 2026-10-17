import { useEffect, useId, useRef, useState, type ReactNode } from 'react';
import { Icon } from '../../../shared/icons/icon.tsx';
import { getIcon, pickerCategories } from '../../../shared/icons/index.ts';

type Props = {
  /** The chosen icon id; empty = none chosen, so the default shows. */
  value: string;
  /** What the default looks like; shown when nothing is chosen and beside "Default". */
  defaultIcon: ReactNode;
  onChange: (icon: string) => void;
  disabled?: boolean;
  /** Accessible name of the button, for example "Workspace icon". */
  label: string;
};

type Anchor = { top: number; left: number; maxHeight: number };

/** The pencil that tells the button can be changed. */
const EDIT_BADGE_ICON_ID = '01yoo4';

const PANEL_GAP = 4;
const PANEL_MAX_HEIGHT = 320;
const SCREEN_MARGIN = 12;

function anchorBelow(button: HTMLElement): Anchor {
  const rect = button.getBoundingClientRect();
  const top = rect.bottom + PANEL_GAP;
  return {
    top,
    left: rect.left,
    maxHeight: Math.max(120, Math.min(PANEL_MAX_HEIGHT, window.innerHeight - top - SCREEN_MARGIN)),
  };
}

/**
 * An icon button (a pencil badge at its bottom-right says it can be changed) that opens a grid of the built-in icons, grouped by category. Esc and a click
 * outside close only the grid, so a dialog around it stays open. The grid is fixed-positioned so
 * a scrolling dialog body does not clip it.
 */
export function IconPicker({ value, defaultIcon, onChange, disabled, label }: Props) {
  const [anchor, setAnchor] = useState<Anchor | null>(null);
  const buttonRef = useRef<HTMLButtonElement | null>(null);
  const panelRef = useRef<HTMLDivElement | null>(null);
  const panelId = useId();
  const open = anchor !== null;
  const chosen = value && getIcon(value) ? value : '';

  useEffect(() => {
    if (!open) return undefined;
    const close = () => setAnchor(null);
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      e.preventDefault();
      e.stopImmediatePropagation();
      close();
      buttonRef.current?.focus();
    };
    const onClick = (e: MouseEvent) => {
      const target = e.target as Node | null;
      if (target && (panelRef.current?.contains(target) || buttonRef.current?.contains(target))) return;
      e.preventDefault();
      e.stopPropagation();
      close();
    };
    const onScroll = (e: Event) => {
      if (!panelRef.current?.contains(e.target as Node | null)) close();
    };
    window.addEventListener('keydown', onKey, true);
    document.addEventListener('click', onClick, true);
    document.addEventListener('scroll', onScroll, true);
    window.addEventListener('resize', close);
    return () => {
      window.removeEventListener('keydown', onKey, true);
      document.removeEventListener('click', onClick, true);
      document.removeEventListener('scroll', onScroll, true);
      window.removeEventListener('resize', close);
    };
  }, [open]);

  function pick(icon: string) {
    onChange(icon);
    setAnchor(null);
    buttonRef.current?.focus();
  }

  return (
    <div className="icon-picker">
      <button
        ref={buttonRef}
        type="button"
        className="icon-picker-button"
        aria-label={label}
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-controls={open ? panelId : undefined}
        disabled={disabled}
        onClick={() => {
          if (open) setAnchor(null);
          else if (buttonRef.current) setAnchor(anchorBelow(buttonRef.current));
        }}
      >
        <span className="icon-picker-main">{chosen ? <Icon id={chosen} size={20} /> : defaultIcon}</span>
        <span className="icon-picker-edit-badge" data-icon-badge="edit" aria-hidden="true">
          <Icon id={EDIT_BADGE_ICON_ID} size={9} />
        </span>
      </button>
      {anchor ? (
        <div
          ref={panelRef}
          id={panelId}
          className="icon-picker-panel"
          role="dialog"
          aria-label="Choose an icon"
          style={{ top: anchor.top, left: anchor.left, maxHeight: anchor.maxHeight }}
        >
          <button
            type="button"
            className={`icon-picker-default${chosen ? '' : ' is-selected'}`}
            data-icon-default="true"
            aria-pressed={!chosen}
            onClick={() => pick('')}
          >
            <span className="icon-picker-default-preview">{defaultIcon}</span>
            <span>Default</span>
          </button>
          {pickerCategories().map((category) => (
            <section key={category.code} className="icon-picker-category" data-icon-category={category.code}>
              <h4 className="icon-picker-category-title">{category.name}</h4>
              <div className="icon-picker-grid">
                {category.icons.map((icon) => (
                  <button
                    key={icon.id}
                    type="button"
                    className={`icon-picker-cell${icon.id === chosen ? ' is-selected' : ''}`}
                    data-icon-id={icon.id}
                    aria-label={icon.name}
                    aria-pressed={icon.id === chosen}
                    data-tip={icon.name}
                    onClick={() => pick(icon.id)}
                  >
                    <Icon id={icon.id} size={18} />
                  </button>
                ))}
              </div>
            </section>
          ))}
        </div>
      ) : null}
    </div>
  );
}
