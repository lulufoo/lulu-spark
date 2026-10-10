import { useEffect, useRef, useState, type RefObject } from 'react';

export type ComposerListKey = 'workspace' | 'staged';

type DetailsToggle = { currentTarget: HTMLDetailsElement };

export function useExclusiveDetailsPair() {
  const [openKey, setOpenKey] = useState<ComposerListKey | null>(null);
  const workspaceRef = useRef<HTMLDetailsElement>(null);
  const stagedRef = useRef<HTMLDetailsElement>(null);

  useEffect(() => {
    if (!openKey) return undefined;
    function isInside(target: EventTarget | null) {
      return Boolean(
        target instanceof Node &&
          (workspaceRef.current?.contains(target) ||
            stagedRef.current?.contains(target)),
      );
    }
    function onPointerDown(event: PointerEvent) {
      if (isInside(event.target)) return;
      setOpenKey(null);
    }
    function onFocusIn(event: FocusEvent) {
      if (isInside(event.target)) return;
      setOpenKey(null);
    }
    document.addEventListener('pointerdown', onPointerDown);
    document.addEventListener('focusin', onFocusIn);
    return () => {
      document.removeEventListener('pointerdown', onPointerDown);
      document.removeEventListener('focusin', onFocusIn);
    };
  }, [openKey]);

  function bind(key: ComposerListKey): {
    open: boolean;
    rootRef: RefObject<HTMLDetailsElement | null>;
    onToggle: (event: DetailsToggle) => void;
  } {
    return {
      open: openKey === key,
      rootRef: key === 'workspace' ? workspaceRef : stagedRef,
      onToggle: (event: DetailsToggle) => {
        const nextOpen = event.currentTarget.open;
        setOpenKey((prev) => {
          if (nextOpen) return key;
          return prev === key ? null : prev;
        });
      },
    };
  }

  return { bind };
}
