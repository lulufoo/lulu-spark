import { useRef, useState, type PointerEvent as ReactPointerEvent, type ReactNode } from 'react';
import { insertionIndex, moveItem } from './reorder.ts';

export type ReorderRow = { id: string; content: ReactNode };

type Drag = { id: string; at: number };

/**
 * A list whose rows can be dragged into another order, only by the handle at the row's left so
 * the row itself keeps its click. A list of one row has nothing to reorder and shows no handle.
 * `onReorder` gets every id in the new order, and only when the order really changed.
 */
export function ReorderRows({
  rows,
  onReorder,
  disabled = false,
}: {
  rows: ReorderRow[];
  onReorder: (ids: string[]) => void;
  disabled?: boolean;
}) {
  const host = useRef<HTMLDivElement | null>(null);
  const [drag, setDrag] = useState<Drag | null>(null);
  const ids = rows.map((row) => row.id);
  const others = drag ? ids.filter((id) => id !== drag.id) : ids;

  const begin = (event: ReactPointerEvent, id: string) => {
    if (disabled || event.button !== 0 || !host.current) return;
    event.preventDefault();
    const list = host.current;
    const rest = ids.filter((x) => x !== id);
    const at = (y: number) => {
      const children = Array.from(list.children) as HTMLElement[];
      const mids = rest.map((otherId) => {
        const box = children[ids.indexOf(otherId)]?.getBoundingClientRect();
        return box ? box.top + box.height / 2 : 0;
      });
      return insertionIndex(mids, y);
    };
    setDrag({ id, at: ids.indexOf(id) });
    const onMove = (e: PointerEvent) => setDrag({ id, at: at(e.clientY) });
    const stop = () => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
      window.removeEventListener('pointercancel', onCancel);
      setDrag(null);
    };
    const onUp = (e: PointerEvent) => {
      stop();
      const next = moveItem(ids, id, at(e.clientY));
      if (next.some((x, i) => x !== ids[i])) onReorder(next);
    };
    const onCancel = () => stop();
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
    window.addEventListener('pointercancel', onCancel);
  };

  const showHandles = rows.length > 1;
  return (
    <div className="reorder-rows" ref={host}>
      {rows.map((row) => {
        const slot = drag ? others.indexOf(row.id) : -1;
        const classes = ['reorder-row'];
        if (drag?.id === row.id) classes.push('is-dragging');
        if (drag && slot === drag.at) classes.push('drop-before');
        if (drag && slot !== -1 && slot === others.length - 1 && drag.at === others.length) classes.push('drop-after');
        return (
          <div key={row.id} className={classes.join(' ')} data-reorder-id={row.id}>
            {showHandles ? (
              <span
                className="reorder-handle"
                role="button"
                aria-label="Drag to reorder"
                title="Drag to reorder"
                onPointerDown={(event) => begin(event, row.id)}
              >
                <svg width="10" height="16" viewBox="0 0 10 16" aria-hidden="true">
                  {[3, 8, 13].flatMap((y) => [2.5, 7.5].map((x) => <circle key={`${x}-${y}`} cx={x} cy={y} r="1.3" />))}
                </svg>
              </span>
            ) : null}
            <div className="reorder-row-body">{row.content}</div>
          </div>
        );
      })}
    </div>
  );
}
