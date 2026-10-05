import type { ReactNode } from 'react';

/** Overlay chrome row under the traffic lights. Optional children sit on the right. */
export function WindowDragStrip({ children }: { children?: ReactNode }) {
  return (
    <div
      className={`window-drag-strip${children ? ' is-chrome' : ''}`}
      data-tauri-drag-region="deep"
    >
      {children}
    </div>
  );
}
