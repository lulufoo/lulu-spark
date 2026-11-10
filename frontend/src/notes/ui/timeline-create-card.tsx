import { useEffect, useRef, useSyncExternalStore } from 'react';
import { isDirectoryView } from '../commands/workspace.ts';
import {
  cancelTimelineCreate,
  commitTimelineCreate,
  setTimelineCreateTitle,
} from '../commands/timeline-create.ts';
import { useHostState } from '../state/host.ts';
import { timelineCreateStore } from '../state/timeline-create.ts';

/** A document card whose only job is to take a title. Enter or blur creates; a blank title cancels. */
export function TimelineCreateCard() {
  const host = useHostState();
  const snap = useSyncExternalStore(timelineCreateStore.subscribe, timelineCreateStore.getSnapshot);
  const inputRef = useRef<HTMLInputElement>(null);
  const directory = isDirectoryView();

  useEffect(() => {
    if (!snap.project || directory) return;
    inputRef.current?.focus();
  }, [snap.tick, snap.project, directory]);

  if (!snap.project || directory) return null;
  const name = host.index.topicTitles[snap.project] || snap.project;
  const pending = host.index.topicDefault?.[snap.project] === true;
  return (
    <div
      className={pending ? 'doc-card default-project-pending' : 'doc-card'}
      data-role="timeline-create"
    >
      <div className="doc-topic">{name}</div>
      <input
        ref={inputRef}
        className="doc-title-btn timeline-create-title"
        aria-label="Document title"
        placeholder="Title"
        value={snap.title}
        disabled={snap.saving}
        onChange={(e) => setTimelineCreateTitle(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Escape') {
            e.preventDefault();
            cancelTimelineCreate();
            return;
          }
          if (e.key === 'Enter') {
            e.preventDefault();
            void commitTimelineCreate();
          }
        }}
        onBlur={() => {
          void commitTimelineCreate();
        }}
      />
    </div>
  );
}
