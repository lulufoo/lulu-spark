import { useEffect, useRef, useState } from 'react';
import type { HubSession } from '../state/store.ts';
import { SessionList } from './session-list.tsx';

/** Scrolling this close to the bottom of the popover list reveals more chats. */
const LOAD_MORE_PX = 40;
const EDGE_GAP_PX = 8;
const MIN_ROOM_PX = 160;

type PanelBox = { left: number; bottom: number; maxHeight: number };

/**
 * The history icon beside "+" in the Chats header. It opens a popover beside the sidebar
 * with every chat; picking one switches to it and closes the popover. The panel is fixed-position
 * because the sidebar clips its own overflow.
 */
export function SessionHistory({
  total,
  sessions,
  currentSessionId,
  progressByChat,
  inFlightIds,
  onSelect,
  onDelete,
  onNearEnd,
}: {
  total: number;
  sessions: HubSession[];
  currentSessionId: string;
  progressByChat: Record<string, string>;
  inFlightIds: string[];
  onSelect: (id: string) => void;
  onDelete: (id: string) => void;
  onNearEnd: () => void;
}) {
  const [box, setBox] = useState<PanelBox | null>(null);
  const triggerRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    if (!box) return undefined;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setBox(null);
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [box]);

  function open() {
    const trigger = triggerRef.current;
    if (!trigger) return;
    const row = trigger.getBoundingClientRect();
    const side = trigger.closest('.home-chat-sidebar')?.getBoundingClientRect();
    setBox({
      left: (side?.right ?? row.right) + EDGE_GAP_PX,
      bottom: window.innerHeight - row.bottom,
      maxHeight: Math.max(MIN_ROOM_PX, row.bottom - EDGE_GAP_PX * 2),
    });
  }

  return (
    <div className="home-history" data-role="history">
      <button
        ref={triggerRef}
        type="button"
        className="home-history-trigger"
        data-role="history-trigger"
        aria-haspopup="dialog"
        aria-expanded={box ? 'true' : 'false'}
        aria-label={`All chats (${total})`}
        data-tip="All chats"
        onClick={() => (box ? setBox(null) : open())}
      >
        <svg className="home-history-icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M3 12a9 9 0 1 0 3-6.7L3 8" />
          <path d="M3 3v5h5" />
          <path d="M12 7v5l3 2" />
        </svg>
      </button>
      {box ? (
        <>
          <div className="home-history-backdrop" data-role="close-history" onClick={() => setBox(null)} />
          <div
            className="home-history-panel"
            data-role="history-panel"
            role="dialog"
            aria-label="All chats"
            style={{ left: box.left, bottom: box.bottom, maxHeight: box.maxHeight }}
          >
            <div className="home-history-head">All chats</div>
            <div
              className="home-history-list"
              data-role="history-list"
              role="list"
              onScroll={(e) => {
                const el = e.currentTarget;
                if (el.scrollHeight - el.scrollTop - el.clientHeight <= LOAD_MORE_PX) onNearEnd();
              }}
            >
              <SessionList
                sessions={sessions}
                currentSessionId={currentSessionId}
                progressByChat={progressByChat}
                inFlightIds={inFlightIds}
                onSelect={(id) => {
                  setBox(null);
                  onSelect(id);
                }}
                onDelete={onDelete}
              />
            </div>
          </div>
        </>
      ) : null}
    </div>
  );
}
