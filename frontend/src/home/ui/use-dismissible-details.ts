import { useEffect, useRef, useState } from 'react';

export function useDismissibleDetails() {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDetailsElement>(null);

  useEffect(() => {
    if (!open) return undefined;
    function isInside(target: EventTarget | null) {
      return Boolean(
        rootRef.current && target instanceof Node && rootRef.current.contains(target),
      );
    }
    function onPointerDown(event: PointerEvent) {
      if (isInside(event.target)) return;
      setOpen(false);
    }
    function onFocusIn(event: FocusEvent) {
      if (isInside(event.target)) return;
      setOpen(false);
    }
    document.addEventListener('pointerdown', onPointerDown);
    document.addEventListener('focusin', onFocusIn);
    return () => {
      document.removeEventListener('pointerdown', onPointerDown);
      document.removeEventListener('focusin', onFocusIn);
    };
  }, [open]);

  return {
    open,
    rootRef,
    onToggle: (event: { currentTarget: HTMLDetailsElement }) => {
      setOpen(event.currentTarget.open);
    },
  };
}
