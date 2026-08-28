import { useEffect, useRef, useSyncExternalStore } from 'react';
import { createModuleStore } from '../shared/module-store.ts';
import { mountReadLaterList } from './list.tsx';

const openStore = createModuleStore(false);

export function closeReadLaterDialog() {
  openStore.set(false);
  document.getElementById('read-later-dialog')?.classList.remove('open');
}

export function openReadLaterDialog() {
  openStore.set(true);
  document.getElementById('read-later-dialog')?.classList.add('open');
}

export function ReadLaterDialog() {
  const open = useSyncExternalStore(openStore.subscribe, openStore.getSnapshot);
  const bodyRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!open || !bodyRef.current) return;
    const { unmount } = mountReadLaterList(bodyRef.current, {
      showTabs: true,
      initialFilter: 'unread',
    });
    return () => unmount();
  }, [open]);

  return (
    <div
      id="read-later-dialog"
      className={open ? 'open' : undefined}
      onClick={(e) => {
        if (e.target === e.currentTarget) closeReadLaterDialog();
      }}
    >
      <div id="read-later-dialog-box">
        <div id="read-later-dialog-header">
          <h3>Read Later</h3>
          <button
            type="button"
            id="btn-read-later-close"
            className="md-header-btn"
            aria-label="Close"
            onClick={() => closeReadLaterDialog()}
          >
            ×
          </button>
        </div>
        <div id="read-later-dialog-body" ref={bodyRef}></div>
      </div>
    </div>
  );
}
