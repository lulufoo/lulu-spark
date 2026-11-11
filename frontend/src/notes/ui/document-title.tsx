import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { hasPendingDocumentTitleSave, saveDocumentTitle } from '../commands/document-title.ts';
import { getEntryId } from '../state/host.ts';
import type { HostNoteEntry } from '../../host/snapshot-types.ts';

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/** The stored timestamp has no timezone offset; display its digits without conversion. */
export function documentCreatedLabel(value?: string): string {
  const match = /^(\d{4})(\d{2})(\d{2})(?:(\d{2})(\d{2})(?:\d{2})?)?$/.exec(value || '');
  if (!match) return 'Created · Unknown';
  const [, year, month, day, hour, minute] = match;
  const monthName = MONTHS[Number(month) - 1];
  if (!monthName || Number(day) < 1 || Number(day) > 31 || (hour && Number(hour) > 23) || (minute && Number(minute) > 59)) {
    return 'Created · Unknown';
  }
  const time = hour && minute ? ` ${hour}:${minute}` : '';
  return `Created · ${monthName} ${Number(day)}, ${year}${time}`;
}

export function NotesDocumentTitle({ entry }: { entry: HostNoteEntry }) {
  const id = getEntryId(entry) || '';
  const savedTitle = entry.title || '';
  const [draft, setDraft] = useState(savedTitle);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');
  const alive = useRef(true);
  const inFlight = useRef(false);
  const cancelled = useRef(false);
  const dirty = useRef(false);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  useEffect(() => {
    if (!dirty.current) { setDraft(savedTitle); setError(''); }
  }, [id, savedTitle]);
  useLayoutEffect(() => {
    const input = inputRef.current;
    if (!input) return undefined;
    const resize = () => {
      input.style.height = 'auto';
      input.style.height = `${input.scrollHeight}px`;
    };
    resize();
    const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(resize);
    if (input.parentElement) observer?.observe(input.parentElement);
    return () => observer?.disconnect();
  }, [draft]);

  async function commit() {
    if (cancelled.current) { cancelled.current = false; return; }
    if (inFlight.current) return;
    if (!dirty.current) return;
    const next = draft.trim();
    if (next === savedTitle && !hasPendingDocumentTitleSave(id)) {
      dirty.current = false;
      setDraft(savedTitle); setError(''); return;
    }
    if (!next) { setError('Title cannot be empty'); return; }
    inFlight.current = true;
    setSaving(true);
    setError('');
    try {
      const saved = await saveDocumentTitle(id, next);
      if (alive.current) { dirty.current = false; setDraft(saved); }
    } catch (reason) {
      const message = reason instanceof Error ? reason.message : 'Could not save title';
      if (alive.current) setError(message);
      else alert(`Could not save title “${next}”: ${message}`);
    } finally {
      inFlight.current = false;
      if (alive.current) setSaving(false);
    }
  }

  return (
    <section className="notes-document-title" aria-label="Document title and creation time">
      <textarea
        ref={inputRef}
        className="notes-document-title-input"
        aria-label="Document title"
        aria-invalid={Boolean(error)}
        placeholder="Add a title"
        rows={1}
        value={draft}
        disabled={!id || saving}
        onChange={(event) => { dirty.current = true; setDraft(event.target.value.replace(/[\r\n]+/g, ' ')); setError(''); }}
        onBlur={() => void commit()}
        onKeyDown={(event) => {
          if (event.key === 'Enter' && !event.nativeEvent.isComposing) {
            event.preventDefault();
            event.currentTarget.blur();
          } else if (event.key === 'Escape') {
            event.preventDefault();
            cancelled.current = true;
            dirty.current = false;
            setDraft(savedTitle);
            setError('');
            event.currentTarget.blur();
          }
        }}
      />
      <div className="notes-document-created">
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
          <rect x="3" y="5" width="18" height="16" rx="2" />
          <path d="M7 3v4M17 3v4M3 10h18" />
        </svg>
        {documentCreatedLabel(entry.created_at)}
      </div>
      {error ? <div className="notes-document-title-error" role="alert">{error}</div> : null}
    </section>
  );
}
