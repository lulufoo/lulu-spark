import { useEffect, useRef, useSyncExternalStore } from 'react';
import { categoryLabel, docCategoryCatalogStore } from '../../host/doc-categories-catalog.ts';
import { notifyState, state } from '../state/host.ts';
import { LAYERS } from '../../host/constants.ts';
import { timeFromTs } from '../../shared/utils.ts';
import { openMoveProjectDialog } from './move-project-dialog.tsx';
import { attachDigestTooltip } from './digest-tooltip.tsx';
import { renderToHtml } from '../../island.ts';
import { cycleImportance, loadTitles, openSourceTypePicker, toggleDone } from '../commands/cards.ts';
import { openNoteInChat } from '../commands/open-in-chat.ts';
import { ViewerHeaderIcon } from '../../shared/viewer-header-icons.tsx';
import type { NoteEntry, NoteTag } from '../state/types.ts';

export { cycleImportance, loadTitles, toggleDone };

function projectKeyOf(entry: NoteEntry): string {
  return entry.project_id || 'unknown';
}

function catalogTopicLabel(entry: NoteEntry): string {
  const key = projectKeyOf(entry);
  const title = state.index.topicTitles[key];
  return title && title !== key ? title : key;
}

function isInboxCard(entry: NoteEntry): boolean {
  return state.index.topicInbox?.[projectKeyOf(entry)] === true;
}

function asRecord(entry: NoteEntry): Record<string, unknown> {
  return entry as unknown as Record<string, unknown>;
}

function isUnreachable(entry: NoteEntry, layer: string): boolean {
  return Boolean(asRecord(entry)[`_unreachable_${layer}`]);
}

function TagBadges({ tags }: { tags?: NoteTag[] }) {
  if (!tags?.length) return null;
  return (
    <>
      {tags.map((tag, i) => {
        if (tag.unknown) {
          return (
            <span key={`u-${tag.value || tag.key || i}`} className="badge badge-tag tag-unknown" data-tip="Unknown tag">
              🏷 {tag.value || tag.key || ''}
            </span>
          );
        }
        const key = tag.key || '';
        return (
          <button
            key={key || i}
            type="button"
            className="badge badge-tag"
            data-tag-key={key}
          >
            🏷 {tag.value || key}
          </button>
        );
      })}
    </>
  );
}

export function tagsBadgesHtml(entry: NoteEntry): string {
  if (!entry.tags?.length) return '';
  return renderToHtml(<TagBadges tags={entry.tags} />);
}

export function updateCardTagsBadge(_entry: NoteEntry) {
  notifyState();
}

export function getEntryDiffState(entry: NoteEntry): 'unreachable' | null {
  for (const layer of LAYERS) {
    if (!entry.layers?.includes(layer)) continue;
    if (isUnreachable(entry, layer)) return 'unreachable';
  }
  return null;
}

function sourceTypeCssClass(id: string) {
  return id === 'theme-line' ? 'themeline' : id;
}

function SourceTypeBadge({ sourceType }: { sourceType?: string }) {
  const label = categoryLabel(sourceType);
  if (!label || !sourceType) return null;
  const cssClass = sourceTypeCssClass(sourceType);
  return (
    <button
      type="button"
      className={`badge badge-source badge-source-${cssClass}`}
      data-action="set-source-type"
      data-tip="Change category"
      aria-label="Change category"
      aria-haspopup="listbox"
      aria-expanded="false"
    >
      {label}
    </button>
  );
}

export function sourceTypeBadgeHtml(sourceType?: string): string {
  if (!categoryLabel(sourceType)) return '';
  return renderToHtml(<SourceTypeBadge sourceType={sourceType} />);
}

function LinksBadge({ links }: { links?: unknown[] }) {
  const linkCount = links && links.length;
  if (!linkCount) return null;
  return (
    <span className="badge badge-links" data-tip={`${linkCount} linked items`}>
      👍 ×{linkCount}
    </span>
  );
}

function OpenInChatBadge() {
  return (
    <button
      type="button"
      className="badge badge-open-in-chat"
      data-action="open-in-chat"
      data-tip="Open in chat"
      aria-label="Open in chat"
    >
      <ViewerHeaderIcon name="chat" filled />
    </button>
  );
}

function startOpenInChat(entry: NoteEntry, button?: HTMLButtonElement | null) {
  void openNoteInChat(entry, button).catch((err) => {
    alert(err instanceof Error ? err.message : String(err));
  });
}

function ImportanceBadge({ importance }: { importance?: string }) {
  const map: Record<string, { cls: string; label: string }> = {
    high: { cls: 'badge-importance-high', label: '↑ High' },
    medium: { cls: 'badge-importance-medium', label: '→ Medium' },
    low: { cls: 'badge-importance-low', label: '↓ Low' },
  };
  const m = importance ? map[importance] : undefined;
  if (m) {
    return (
      <button className={`badge badge-importance ${m.cls}`} data-action="cycle-importance" data-tip="Cycle importance">
        {m.label}
      </button>
    );
  }
  return (
    <button className="badge badge-importance badge-importance-unset" data-action="cycle-importance" data-tip="Set importance">
      ☆
    </button>
  );
}

function DoneBadge({ done }: { done?: boolean }) {
  return done ? (
    <button className="badge badge-done" data-action="toggle-done" data-tip="Mark as not done">
      ✓ Done
    </button>
  ) : (
    <button className="badge badge-done" data-action="toggle-done" data-tip="Mark as done">
      ○ Mark done
    </button>
  );
}

function MoveBadge() {
  return (
    <button className="badge badge-move-project" data-action="move-project" data-tip="Move to another project">
      <span className="move-icon">↳</span>
      <span>Move project</span>
    </button>
  );
}

function DiffDot({ diffState }: { diffState: 'unreachable' | null }) {
  if (!diffState) return null;
  return (
    <>
      {' '}
      <span className={`diff-dot ${diffState}`}>● unreachable</span>
    </>
  );
}

function CardInner({
  entry,
  displayTitle,
  topic,
  topicDesc,
  time,
}: {
  entry: NoteEntry;
  displayTitle: string | null;
  topic: string;
  topicDesc: string;
  time: string;
}) {
  const diffState = getEntryDiffState(entry);
  return (
    <>
      <div className="doc-topic" data-tip={topicDesc || undefined}>
        {topic}
      </div>
      <button className={`doc-title-btn${displayTitle === null ? ' loading' : ''}`}>
        {displayTitle !== null ? displayTitle : ''}
      </button>
      <div className="doc-meta">
        {time}
        <DiffDot diffState={diffState} />
      </div>
      <div className="badges">
        <SourceTypeBadge sourceType={entry.source_type} />
        <TagBadges tags={entry.tags} />
        <LinksBadge links={entry.links} />
        <span className="doc-card-icon-actions">
          <ImportanceBadge importance={entry.importance} />
          <OpenInChatBadge />
        </span>
        <DoneBadge done={!!entry.done} />
        <MoveBadge />
      </div>
    </>
  );
}

function bindCardActions(card: HTMLElement, entry: NoteEntry) {
  const firstLayer = LAYERS.find((l) => entry.layers?.includes(l)) || 'raw';
  card.querySelector('.doc-title-btn')?.addEventListener('click', () => {
    document.dispatchEvent(
      new CustomEvent('cta:open-entry', {
        detail: { id: entry._id || entry.id, layer: firstLayer },
      }),
    );
  });
  card.querySelector('[data-action="toggle-done"]')?.addEventListener('click', () => toggleDone(entry));
  card.querySelector('[data-action="cycle-importance"]')?.addEventListener('click', () => cycleImportance(entry));
  card.querySelector('[data-action="move-project"]')?.addEventListener('click', () => openMoveProjectDialog(entry));
  card.querySelector('[data-action="set-source-type"]')?.addEventListener('click', (e) => {
    e.stopPropagation();
    openSourceTypePicker(e.currentTarget as HTMLElement, entry);
  });
  card.querySelector('[data-action="open-in-chat"]')?.addEventListener('click', (e) => {
    e.stopPropagation();
    startOpenInChat(entry, e.currentTarget as HTMLButtonElement);
  });
}

export function buildCard(id: string, entry: NoteEntry, title?: string | null) {
  const card = document.createElement('div');
  card.className = 'doc-card';
  card.dataset.id = id;
  entry._id = id;

  const topic = catalogTopicLabel(entry);
  const displayTitle = title !== undefined ? title : null;
  const time = timeFromTs(entry.created_at || '');
  const projectDir = projectKeyOf(entry);
  const topicDesc = state.index.topicDescriptions[projectDir] || '';

  card.innerHTML = renderToHtml(
    <CardInner
      entry={entry}
      displayTitle={displayTitle}
      topic={topic}
      topicDesc={topicDesc}
      time={time}
    />,
  );
  if (entry.done) card.classList.add('done');
  if (entry.importance) card.classList.add(`importance-${entry.importance}`);
  if (isInboxCard(entry)) card.classList.add('inbox-pending');
  bindCardActions(card, entry);
  attachCardDigestTooltip(card, id);
  return card;
}

function attachCardDigestTooltip(root: HTMLElement, id: string): (() => void) | undefined {
  const badges = root.querySelector('.badges');
  if (!(badges instanceof HTMLElement)) return;
  return attachDigestTooltip(badges, id);
}

function openEntry(entry: NoteEntry) {
  const firstLayer = LAYERS.find((l) => entry.layers?.includes(l)) || 'raw';
  document.dispatchEvent(
    new CustomEvent('cta:open-entry', {
      detail: { id: entry._id || entry.id, layer: firstLayer },
    }),
  );
}

export function DocCard({
  id,
  entry,
  title,
}: {
  id: string;
  entry: NoteEntry;
  title: string | null;
}) {
  useSyncExternalStore(
    docCategoryCatalogStore.subscribe,
    docCategoryCatalogStore.getSnapshot,
    docCategoryCatalogStore.getSnapshot,
  );
  const cardRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const el = cardRef.current;
    if (!el) return;
    return attachCardDigestTooltip(el, id);
  }, [id]);

  entry._id = id;
  const topic = catalogTopicLabel(entry);
  const time = timeFromTs(entry.created_at || '');
  const projectDir = projectKeyOf(entry);
  const topicDesc = state.index.topicDescriptions[projectDir] || '';
  const className = [
    'doc-card',
    entry.done ? 'done' : '',
    entry.importance ? `importance-${entry.importance}` : '',
    isInboxCard(entry) ? 'inbox-pending' : '',
  ]
    .filter(Boolean)
    .join(' ');

  function onClick(e: { target: EventTarget | null; stopPropagation: () => void }) {
    const t = (e.target as HTMLElement | null)?.closest?.(
      '[data-action], .doc-title-btn',
    ) as HTMLElement | null;
    if (!t) return;
    if (t.dataset.action === 'toggle-done') {
      void toggleDone(entry);
      return;
    }
    if (t.dataset.action === 'cycle-importance') {
      void cycleImportance(entry);
      return;
    }
    if (t.dataset.action === 'move-project') {
      openMoveProjectDialog(entry);
      return;
    }
    if (t.dataset.action === 'set-source-type') {
      e.stopPropagation();
      openSourceTypePicker(t, entry);
      return;
    }
    if (t.dataset.action === 'open-in-chat') {
      e.stopPropagation();
      startOpenInChat(entry, t as HTMLButtonElement);
      return;
    }
    if (t.classList.contains('doc-title-btn')) {
      openEntry(entry);
    }
  }

  return (
    <div ref={cardRef} className={className} data-id={id} onClick={onClick}>
      <CardInner entry={entry} displayTitle={title} topic={topic} topicDesc={topicDesc} time={time} />
    </div>
  );
}

export function NotesDocList({
  entries,
  date,
  empty,
  emptyMessage,
}: {
  entries: { id: string; entry: NoteEntry }[];
  date: string;
  empty?: boolean;
  /** Shown when `empty`; without one the list stays blank (the toolbar heading already says it). */
  emptyMessage?: string;
}) {
  if (empty) {
    return emptyMessage ? <div className="empty-filter-msg">{emptyMessage}</div> : null;
  }

  return (
    <>
      {entries.map(({ id, entry }) => {
        const cached = date ? state.index.titleCache.get(date)?.get(id) : undefined;
        const initialTitle = cached ?? (entry.title || 'Untitled note');
        return <DocCard key={id} id={id} entry={entry} title={initialTitle} />;
      })}
    </>
  );
}

/** Tests / leftover callers: paint cards into `#doc-list`. Production uses NotesDocList. */
export function renderDocList(entries: { id: string; entry: NoteEntry }[], date: string) {
  const list = document.getElementById('doc-list');
  if (!list) return;
  list.innerHTML = '';

  for (const { id, entry } of entries) {
    const cached = state.index.titleCache.get(date)?.get(id);
    const initialTitle = cached ?? (entry.title || 'Untitled note');
    const card = buildCard(id, entry, initialTitle);
    list.appendChild(card);
  }

  const savedScroll = sessionStorage.getItem('cta_scroll_' + date);
  if (savedScroll) {
    requestAnimationFrame(() => {
      list.scrollTop = parseInt(savedScroll, 10);
    });
  }
}

export function updateTitlesInDOM(date: string) {
  const cache = state.index.titleCache.get(date);
  if (!cache) return;
  const group =
    state.index.groupedByDate.find((g) => g.date === date) ||
    state.index.filteredGroups.find((g) => g.date === date);
  if (!group) return;

  for (const { id, entry } of group.entries) {
    const card = document.querySelector(`.doc-card[data-id="${id}"]`);
    if (!card) continue;

    const titleEl = card.querySelector('.doc-title-btn');
    if (titleEl) {
      titleEl.classList.remove('loading');
      titleEl.textContent = cache.get(id) || (entry.title || 'Untitled note');
    }

    const badgesEl = card.querySelector('.badges');
    if (badgesEl) {
      badgesEl.innerHTML = renderToHtml(
        <>
          <SourceTypeBadge sourceType={entry.source_type} />
          <TagBadges tags={entry.tags} />
          <LinksBadge links={entry.links} />
          <span className="doc-card-icon-actions">
            <ImportanceBadge importance={entry.importance} />
            <OpenInChatBadge />
          </span>
          <DoneBadge done={!!entry.done} />
          <MoveBadge />
        </>,
      );
      card.classList.toggle('done', !!entry.done);
      card.classList.remove('importance-high', 'importance-medium', 'importance-low');
      if (entry.importance) card.classList.add(`importance-${entry.importance}`);
      card.querySelector('[data-action="toggle-done"]')?.addEventListener('click', () => toggleDone(entry));
      card
        .querySelector('[data-action="cycle-importance"]')
        ?.addEventListener('click', () => cycleImportance(entry));
      card.querySelector('[data-action="move-project"]')?.addEventListener('click', () => openMoveProjectDialog(entry));
      card.querySelector('[data-action="set-source-type"]')?.addEventListener('click', (e) => {
        e.stopPropagation();
        openSourceTypePicker(e.currentTarget as HTMLElement, entry);
      });
      card.querySelector('[data-action="open-in-chat"]')?.addEventListener('click', (e) => {
        e.stopPropagation();
        startOpenInChat(entry, e.currentTarget as HTMLButtonElement);
      });
    }
  }
}

export function updateDiffInDOM() {
  if (!state.ui.activeDate) return;
  const group =
    state.index.groupedByDate.find((g) => g.date === state.ui.activeDate) ||
    state.index.filteredGroups.find((g) => g.date === state.ui.activeDate);
  if (!group) return;
  for (const { id, entry } of group.entries) {
    const card = document.querySelector(`.doc-card[data-id="${id}"]`);
    if (!card) continue;
    const metaEl = card.querySelector('.doc-meta');
    if (metaEl) {
      const diffState = getEntryDiffState(entry);
      metaEl.innerHTML = renderToHtml(
        <>
          {timeFromTs(entry.created_at || '')}
          <DiffDot diffState={diffState} />
        </>,
      );
    }
  }
}
