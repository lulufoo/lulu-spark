import { notifyState, state } from '../state/host.ts';
import { LAYERS } from '../../host/constants.ts';
import { slugToTitle, filenameFromPath, topicFromPath, timeFromTs } from '../../shared/utils.ts';
import { openMoveProjectDialog } from './move-project-dialog.tsx';
import { renderToHtml } from '../../island.ts';
import { cycleImportance, loadTitles, toggleDone } from '../commands/cards.ts';
import type { NoteEntry, NoteTag } from '../state/types.ts';

export { cycleImportance, loadTitles, toggleDone };

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
            <span key={`u-${tag.value || tag.key || i}`} className="badge badge-tag tag-unknown" title="Unknown tag">
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
            title="Filter by this tag"
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

function attachTagBadgeListeners(card: Element) {
  card.querySelectorAll('.badge-tag[data-tag-key]').forEach((btn) => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const key = (btn as HTMLElement).dataset.tagKey;
      if (key) {
        document.dispatchEvent(new CustomEvent('cta:filter-tag', { detail: { key } }));
      }
    });
  });
}

export function updateCardTagsBadge(_entry: NoteEntry) {
  notifyState();
}

export function getEntryDiffState(entry: NoteEntry): 'conflict' | 'modified' | null {
  let result: 'modified' | null = null;
  for (const layer of LAYERS) {
    if (!entry.layers?.includes(layer)) continue;
    const s = state.index.diffStatus.get(`${layer}/${entry.common_path}`);
    if (s === 'conflict') return 'conflict';
    if (s === 'modified') result = 'modified';
  }
  return result;
}

export function getLayerBadgeClass(layer: string, entry: NoteEntry): string {
  const s = state.index.diffStatus.get(`${layer}/${entry.common_path}`);
  if (s === 'conflict') return 'badge-conflict';
  if (s === 'modified') return 'badge-diff';
  return 'badge-layer';
}

const SOURCE_TYPE_LABELS: Record<string, string> = {
  dialogue: 'Dialogue',
  summary: 'Summary',
  'theme-line': 'Video',
  jot: 'Jot',
  note: 'Note',
};

function SourceTypeBadge({ sourceType }: { sourceType?: string }) {
  const label = sourceType ? SOURCE_TYPE_LABELS[sourceType] : undefined;
  if (!label || !sourceType) return null;
  const cssClass = sourceType === 'theme-line' ? 'themeline' : sourceType;
  return <span className={`badge badge-source badge-source-${cssClass}`}>{label}</span>;
}

export function sourceTypeBadgeHtml(sourceType?: string): string {
  const label = sourceType ? SOURCE_TYPE_LABELS[sourceType] : undefined;
  if (!label) return '';
  return renderToHtml(<SourceTypeBadge sourceType={sourceType} />);
}

function LayerBadges({
  entry,
  includeCommentCounts = true,
}: {
  entry: NoteEntry;
  includeCommentCounts?: boolean;
}) {
  return (
    <>
      {LAYERS.filter((layer) => entry.layers?.includes(layer)).map((layer) => {
        const cc = includeCommentCounts ? entry._comment_counts?.[layer] : undefined;
        const ccNode = cc ? <span className="badge-comment-dot">{cc}</span> : null;
        if (isUnreachable(entry, layer)) {
          return (
            <span key={layer} className="badge badge-unreachable" title="File unreachable">
              {layer}
              {ccNode}
            </span>
          );
        }
        return (
          <button key={layer} className={`badge ${getLayerBadgeClass(layer, entry)}`} data-layer={layer}>
            {layer}
            {ccNode}
          </button>
        );
      })}
    </>
  );
}

function LinksBadge({ links }: { links?: unknown[] }) {
  const linkCount = links && links.length;
  if (!linkCount) return null;
  return (
    <span className="badge badge-links" title={`${linkCount} linked items`}>
      👍 ×{linkCount}
    </span>
  );
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
      <button className={`badge badge-importance ${m.cls}`} data-action="cycle-importance" title="Cycle importance">
        {m.label}
      </button>
    );
  }
  return (
    <button className="badge badge-importance badge-importance-unset" data-action="cycle-importance" title="Set importance">
      ☆
    </button>
  );
}

function DoneBadge({ done }: { done?: boolean }) {
  return done ? (
    <button className="badge badge-done" data-action="toggle-done" title="Mark as not done">
      ✓ Done
    </button>
  ) : (
    <button className="badge badge-done" data-action="toggle-done" title="Mark as done">
      ○ Mark done
    </button>
  );
}

function MoveBadge() {
  return (
    <button className="badge badge-move-project" data-action="move-project" title="Move to another project">
      <span className="move-icon">↳</span>
      <span>Move project</span>
    </button>
  );
}

function DiffDot({ diffState }: { diffState: 'conflict' | 'modified' | null }) {
  if (!diffState) return null;
  return (
    <>
      {' '}
      <span className={`diff-dot ${diffState}`}>{diffState === 'conflict' ? '● conflict' : '● Pending commit'}</span>
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
        <LayerBadges entry={entry} />
        <TagBadges tags={entry.tags} />
        <LinksBadge links={entry.links} />
        <ImportanceBadge importance={entry.importance} />
        <DoneBadge done={!!entry.done} />
        <MoveBadge />
      </div>
    </>
  );
}

export function attachBadgeListeners(card: Element, entry: NoteEntry) {
  card.querySelectorAll('.badge[data-layer]').forEach((btn) => {
    btn.addEventListener('click', () => {
      document.dispatchEvent(
        new CustomEvent('cta:open-entry', {
          detail: { common_path: entry.common_path, layer: (btn as HTMLElement).dataset.layer },
        }),
      );
    });
  });
}

function bindCardActions(card: HTMLElement, entry: NoteEntry) {
  const firstLayer = LAYERS.find((l) => entry.layers?.includes(l)) || 'raw';
  card.querySelector('.doc-title-btn')?.addEventListener('click', () => {
    document.dispatchEvent(
      new CustomEvent('cta:open-entry', {
        detail: { common_path: entry.common_path, layer: firstLayer },
      }),
    );
  });
  attachBadgeListeners(card, entry);
  attachTagBadgeListeners(card);
  card.querySelector('[data-action="toggle-done"]')?.addEventListener('click', () => toggleDone(entry));
  card.querySelector('[data-action="cycle-importance"]')?.addEventListener('click', () => cycleImportance(entry));
  card.querySelector('[data-action="move-project"]')?.addEventListener('click', () => openMoveProjectDialog(entry));
}

export function buildCard(id: string, entry: NoteEntry, title?: string | null) {
  const card = document.createElement('div');
  card.className = 'doc-card';
  card.dataset.id = id;
  entry._id = id;

  const topic = topicFromPath(entry.common_path);
  const displayTitle = title !== undefined ? title : null;
  const time = timeFromTs(entry.created_at || '');
  const projectDir = entry.common_path.split('/')[0];
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
  if (entry.common_path.split('/')[0] === 'inbox') card.classList.add('inbox-pending');
  bindCardActions(card, entry);
  return card;
}

function openEntry(entry: NoteEntry, layer?: string) {
  const firstLayer = layer || LAYERS.find((l) => entry.layers?.includes(l)) || 'raw';
  document.dispatchEvent(
    new CustomEvent('cta:open-entry', {
      detail: { common_path: entry.common_path, layer: firstLayer },
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
  entry._id = id;
  const topic = topicFromPath(entry.common_path);
  const time = timeFromTs(entry.created_at || '');
  const projectDir = entry.common_path.split('/')[0];
  const topicDesc = state.index.topicDescriptions[projectDir] || '';
  const className = [
    'doc-card',
    entry.done ? 'done' : '',
    entry.importance ? `importance-${entry.importance}` : '',
    projectDir === 'inbox' ? 'inbox-pending' : '',
  ]
    .filter(Boolean)
    .join(' ');

  function onClick(e: { target: EventTarget | null; stopPropagation: () => void }) {
    const t = (e.target as HTMLElement | null)?.closest?.(
      '[data-tag-key], [data-layer], [data-action], .doc-title-btn',
    ) as HTMLElement | null;
    if (!t) return;
    if (t.dataset.tagKey) {
      e.stopPropagation();
      document.dispatchEvent(new CustomEvent('cta:filter-tag', { detail: { key: t.dataset.tagKey } }));
      return;
    }
    if (t.dataset.layer) {
      openEntry(entry, t.dataset.layer);
      return;
    }
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
    if (t.classList.contains('doc-title-btn')) {
      openEntry(entry);
    }
  }

  return (
    <div className={className} data-id={id} onClick={onClick}>
      <CardInner entry={entry} displayTitle={title} topic={topic} topicDesc={topicDesc} time={time} />
    </div>
  );
}

export function NotesDocList({
  entries,
  date,
  empty,
}: {
  entries: { id: string; entry: NoteEntry }[];
  date: string;
  empty?: boolean;
}) {
  if (empty) {
    return <div className="empty-filter-msg">No documents match the current filters</div>;
  }

  return (
    <>
      {entries.map(({ id, entry }) => {
        const cached = date ? state.index.titleCache.get(date)?.get(id) : undefined;
        const initialTitle = cached ?? slugToTitle(filenameFromPath(entry.common_path));
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
    const initialTitle = cached ?? slugToTitle(filenameFromPath(entry.common_path));
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
      titleEl.textContent = cache.get(id) || slugToTitle(filenameFromPath(entry.common_path));
    }

    const badgesEl = card.querySelector('.badges');
    if (badgesEl) {
      badgesEl.innerHTML = renderToHtml(
        <>
          <SourceTypeBadge sourceType={entry.source_type} />
          <LayerBadges entry={entry} />
          <TagBadges tags={entry.tags} />
          <LinksBadge links={entry.links} />
          <ImportanceBadge importance={entry.importance} />
          <DoneBadge done={!!entry.done} />
          <MoveBadge />
        </>,
      );
      attachBadgeListeners(card, entry);
      attachTagBadgeListeners(card);
      card.classList.toggle('done', !!entry.done);
      card.classList.remove('importance-high', 'importance-medium', 'importance-low');
      if (entry.importance) card.classList.add(`importance-${entry.importance}`);
      card.querySelector('[data-action="toggle-done"]')?.addEventListener('click', () => toggleDone(entry));
      card
        .querySelector('[data-action="cycle-importance"]')
        ?.addEventListener('click', () => cycleImportance(entry));
      card.querySelector('[data-action="move-project"]')?.addEventListener('click', () => openMoveProjectDialog(entry));
    }
  }
}

export function updateDiffInDOM() {
  if (!state.ui.activeDate) return;
  const group = state.index.groupedByDate.find((g) => g.date === state.ui.activeDate);
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
    const badgesEl = card.querySelector('.badges');
    if (badgesEl) {
      badgesEl.innerHTML = renderToHtml(<LayerBadges entry={entry} includeCommentCounts={false} />);
      attachBadgeListeners(card, entry);
    }
  }
}
