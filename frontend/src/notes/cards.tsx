// @ts-nocheck — ported from JS; state shapes stay unchecked like checkJs:false.
import { state } from '../host/state.ts';
import { LAYERS, IMPORTANCE_CYCLE } from '../host/constants.ts';
import { slugToTitle, filenameFromPath, topicFromPath, timeFromTs, importanceBadgeHtml } from '../shared/utils.ts';
import * as api from '../host/api.ts';
import { openMoveProjectDialog } from './move-project-dialog.tsx';
import { renderToHtml } from '../island.ts';

type NoteTag = { unknown?: boolean; value?: string; key?: string };

type NoteEntry = {
  _id?: string;
  common_path: string;
  created_at?: string;
  layers?: string[];
  source_type?: string;
  done?: boolean | undefined;
  importance?: string;
  links?: unknown[];
  tags?: NoteTag[];
  translations?: { zh?: string };
  _comment_counts?: Record<string, number>;
};

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

function tagsBadgesHtml(entry: NoteEntry): string {
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

export function updateCardTagsBadge(entry: NoteEntry) {
  if (!entry._id && state.index.data) {
    for (const [eid, e] of Object.entries(state.index.data)) {
      if (e === entry) {
        entry._id = eid;
        break;
      }
    }
  }
  const cardId = entry._id;
  if (!cardId) return;
  const card = document.querySelector(`.doc-card[data-id="${cardId}"]`);
  if (!card) return;
  const badges = card.querySelector('.badges');
  if (!badges) return;
  badges.querySelectorAll('.badge-tag').forEach((el) => el.remove());
  const html = tagsBadgesHtml(entry);
  if (!html) return;
  const wrap = document.createElement('span');
  wrap.innerHTML = html;
  const linksBadge = badges.querySelector('.badge-links');
  const frag = document.createDocumentFragment();
  while (wrap.firstChild) frag.appendChild(wrap.firstChild);
  if (linksBadge) linksBadge.before(frag);
  else badges.insertBefore(frag, badges.querySelector('[data-action="cycle-importance"]') || badges.firstChild);
  attachTagBadgeListeners(card);
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
  card.querySelector('[data-action="toggle-done"]')?.addEventListener('click', () => toggleDone(entry, card));
  card.querySelector('[data-action="cycle-importance"]')?.addEventListener('click', () => cycleImportance(entry, card));
  card.querySelector('[data-action="move-project"]')?.addEventListener('click', () => openMoveProjectDialog(entry));
}

export function buildCard(id: string, entry: NoteEntry, title?: string | null) {
  const card = document.createElement('div');
  card.className = 'doc-card';
  card.dataset.id = id;
  entry._id = id;

  const topic = topicFromPath(entry.common_path);
  const displayTitle = title !== undefined ? title : null;
  const time = timeFromTs(entry.created_at);
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

export async function cycleImportance(entry: NoteEntry, card: HTMLElement) {
  const cur = entry.importance;
  const idx = IMPORTANCE_CYCLE.indexOf(cur);
  const next = IMPORTANCE_CYCLE[(idx + 1) % IMPORTANCE_CYCLE.length];
  try {
    const data = await api.setImportance(entry.common_path, next);
    if (!data.ok) return;
    entry.importance = next;
    card.classList.remove('importance-high', 'importance-medium', 'importance-low');
    if (next) card.classList.add(`importance-${next}`);
    const btn = card.querySelector('[data-action="cycle-importance"]');
    if (btn) {
      btn.outerHTML = importanceBadgeHtml(next);
      card
        .querySelector('[data-action="cycle-importance"]')
        ?.addEventListener('click', () => cycleImportance(entry, card));
    }
  } catch (e) {
    console.error('cycleImportance failed', e);
  }
}

export async function toggleDone(entry: NoteEntry, card: HTMLElement) {
  const newDone = !entry.done;
  try {
    const data = await api.setDone(entry.common_path, newDone);
    if (!data.ok) return;
    entry.done = newDone || undefined;
    card.classList.toggle('done', !!newDone);
    const btn = card.querySelector('[data-action="toggle-done"]');
    if (btn instanceof HTMLElement) {
      btn.textContent = newDone ? '✓ Done' : '○ Mark done';
      btn.title = newDone ? 'Mark as not done' : 'Mark as done';
    }
  } catch (e) {
    console.error('toggleDone failed', e);
  }
}

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

export async function loadTitles(entries: { id: string; entry: NoteEntry }[], date: string) {
  if (!state.index.titleCache.has(date)) {
    state.index.titleCache.set(date, new Map());
  }
  const cache = state.index.titleCache.get(date);
  const pending = entries.filter(({ id }) => !cache.has(id));
  if (pending.length === 0) {
    if (state.ui.activeDate === date) updateTitlesInDOM(date);
    return;
  }

  await Promise.all(
    pending.map(async ({ id, entry }) => {
      if (!entry.layers?.includes('raw')) {
        const title = slugToTitle(filenameFromPath(entry.common_path));
        cache.set(id, title);
        return;
      }

      try {
        const titlePath = entry.translations?.zh || entry.common_path;
        const text = await api.fetchFileContent('raw', titlePath);
        const h1Match = text.match(/^#\s+(.+)/m);
        const title = h1Match ? h1Match[1].trim() : slugToTitle(filenameFromPath(entry.common_path));
        cache.set(id, title);
      } catch {
        asRecord(entry)._unreachable_raw = true;
        cache.set(id, slugToTitle(filenameFromPath(entry.common_path)));
      }
    }),
  );

  if (state.ui.activeDate === date) updateTitlesInDOM(date);
}

export function updateTitlesInDOM(date: string) {
  const cache = state.index.titleCache.get(date);
  if (!cache) return;
  const group =
    state.index.groupedByDate.find((g: { date: string }) => g.date === date) ||
    state.index.filteredGroups.find((g: { date: string }) => g.date === date);
  if (!group) return;

  for (const { id, entry } of group.entries as { id: string; entry: NoteEntry }[]) {
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
      card.querySelector('[data-action="toggle-done"]')?.addEventListener('click', () => toggleDone(entry, card as HTMLElement));
      card
        .querySelector('[data-action="cycle-importance"]')
        ?.addEventListener('click', () => cycleImportance(entry, card as HTMLElement));
      card.querySelector('[data-action="move-project"]')?.addEventListener('click', () => openMoveProjectDialog(entry));
    }
  }
}

export function updateDiffInDOM() {
  if (!state.ui.activeDate) return;
  const group = state.index.groupedByDate.find((g: { date: string }) => g.date === state.ui.activeDate);
  if (!group) return;
  for (const { id, entry } of group.entries as { id: string; entry: NoteEntry }[]) {
    const card = document.querySelector(`.doc-card[data-id="${id}"]`);
    if (!card) continue;
    const metaEl = card.querySelector('.doc-meta');
    if (metaEl) {
      const diffState = getEntryDiffState(entry);
      metaEl.innerHTML = renderToHtml(
        <>
          {timeFromTs(entry.created_at)}
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
