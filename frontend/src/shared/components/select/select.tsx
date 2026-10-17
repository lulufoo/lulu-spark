import { useLayoutEffect, useRef } from 'react';
import { closeFloatingListSelect, createFloatingListSelect } from '../../floating-list-select.ts';
import type { FloatingListSelectOption } from '../../types.ts';

export type SelectOption = FloatingListSelectOption;

export type SelectProps = {
  value: string;
  options: SelectOption[];
  onChange: (value: string) => void;
  ariaLabel: string;
  /** Height the open menu may take before it scrolls. */
  menuMaxHeight?: number;
};

/**
 * The one dropdown of the app: the look of the Project dropdown (`.ui-select`, its menu
 * `.ui-select-menu`). It wraps `createFloatingListSelect`, which keeps the positioning, the
 * closing and the option list; this component only mounts it and keeps it in step with props.
 * A value that is not among the options shows the first option's label.
 */
export function Select({ value, options, onChange, ariaLabel, menuMaxHeight }: SelectProps) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const handleRef = useRef<ReturnType<typeof createFloatingListSelect> | null>(null);
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;
  const latest = useRef({ value, options });
  latest.current = { value, options };

  useLayoutEffect(() => {
    const host = hostRef.current;
    if (!host) return undefined;
    const handle = createFloatingListSelect({
      ariaLabel,
      pickerClass: 'ui-select',
      menuMaxHeight,
      value: latest.current.value,
      options: latest.current.options,
      onSelect: (next) => onChangeRef.current(next),
    });
    handleRef.current = handle;
    host.appendChild(handle.picker);
    return () => {
      closeFloatingListSelect();
      handle.picker.remove();
      handleRef.current = null;
    };
  }, [ariaLabel, menuMaxHeight]);

  useLayoutEffect(() => {
    handleRef.current?.sync({ value, options });
  }, [value, options]);

  return <div className="ui-select-host" ref={hostRef} />;
}
