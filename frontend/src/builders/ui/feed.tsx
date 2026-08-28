import { useEffect, useState } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { loadBuilderFeeds } from '../commands/feed.ts';
import { fmtDate, fmtDateFull, pickXSelection } from '../state/selectors.ts';
import {
  errMessage,
  type BlogsFeedData,
  type BuilderBlog,
  type BuilderPodcast,
  type BuilderXUser,
  type FeedViewState,
  type PodcastsFeedData,
  type XFeedData,
} from '../state/types.ts';
import { getXHandle, isXCollapsed, saveXCollapsed, saveXHandle } from '../state/x-prefs.ts';

function XUser({ user }: { user: BuilderXUser }) {
  const tweets = user.tweets || [];
  return (
    <div className="feed-person">
      <div className="feed-person-header">
        <span className="feed-person-name">{user.name}</span>
        <a
          className="feed-person-handle"
          href={`https://x.com/${user.handle}`}
          target="_blank"
          rel="noopener"
        >
          @{user.handle} ↗
        </a>
      </div>
      <div className="feed-person-tweets">
        {tweets.length === 0 ? (
          <div className="feed-tweet-empty">No recent tweets</div>
        ) : (
          tweets.map((t, i) => (
            <div className="feed-tweet" key={t.url || i}>
              <div className="feed-tweet-text">{t.text}</div>
              <div className="feed-tweet-meta">
                <span className="feed-tweet-ts">{fmtDate(t.createdAt)}</span>
                {t.likes ? <span className="feed-tweet-stat">♥ {t.likes}</span> : null}
                {t.retweets ? <span className="feed-tweet-stat">🔁 {t.retweets}</span> : null}
                {t.replies ? <span className="feed-tweet-stat">💬 {t.replies}</span> : null}
                <a className="feed-tweet-link" href={t.url} target="_blank" rel="noopener">
                  Original ↗
                </a>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

function PodcastItem({ p, index }: { p: BuilderPodcast; index: number }) {
  const [open, setOpen] = useState(false);
  const id = `feed-podcast-${index}`;
  const preview = p.transcript ? p.transcript.slice(0, 300) : '';
  const hasMore = p.transcript && p.transcript.length > 300;
  return (
    <div className="feed-podcast">
      <div className="feed-podcast-title">
        <a href={p.url} target="_blank" rel="noopener">
          {p.title} ↗
        </a>
      </div>
      <div className="feed-podcast-meta">
        <span className="feed-podcast-show">{p.name}</span>
        <span className="feed-podcast-ts">{fmtDate(p.publishedAt)}</span>
      </div>
      {p.transcript ? (
        <div className="feed-podcast-transcript">
          <div className="feed-transcript-preview" id={`${id}-preview`} style={{ display: open ? 'none' : undefined }}>
            {preview}
            {hasMore ? '…' : ''}
          </div>
          {hasMore ? (
            <>
              <div className="feed-transcript-full" id={`${id}-full`} style={{ display: open ? undefined : 'none' }}>
                {p.transcript}
              </div>
              <button type="button" className="feed-transcript-toggle" data-target={id} onClick={() => setOpen((v) => !v)}>
                {open ? 'Collapse' : 'Expand'}
              </button>
            </>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

function Podcasts({ podcasts }: { podcasts?: BuilderPodcast[] }) {
  if (!podcasts || podcasts.length === 0) {
    return <div className="feed-empty">No podcasts</div>;
  }
  return (
    <>
      {podcasts.map((p, i) => (
        <PodcastItem key={p.url || `feed-podcast-${i}`} p={p} index={i} />
      ))}
    </>
  );
}

function Blogs({ blogs }: { blogs?: BuilderBlog[] }) {
  if (!blogs || blogs.length === 0) {
    return <div className="feed-empty">No blogs yet (blog sources still being configured)</div>;
  }
  return (
    <>
      {blogs.map((b) => (
        <div className="feed-blog" key={b.url || b.title}>
          <a className="feed-blog-title" href={b.url} target="_blank" rel="noopener">
            {b.title} ↗
          </a>
          <div className="feed-blog-meta">
            <span>{b.author || b.name || ''}</span>
            <span>{fmtDate(b.publishedAt)}</span>
          </div>
        </div>
      ))}
    </>
  );
}

function FeedError({ message }: { message?: string }) {
  return <div className="feed-error">Failed to load: {message}</div>;
}

function FeedView({
  generatedAt,
  podcastsData,
  blogsData,
  xData,
  xUsers,
}: {
  generatedAt: string;
  podcastsData: PodcastsFeedData;
  blogsData: BlogsFeedData;
  xData: XFeedData;
  xUsers: BuilderXUser[];
}) {
  const [collapsed, setCollapsed] = useState(isXCollapsed());
  const [handle, setHandle] = useState(getXHandle());
  const { xSelHandle, xSelUser } = pickXSelection(xUsers, handle);

  return (
    <>
      {generatedAt ? <div className="feed-updated">Updated {fmtDateFull(generatedAt)}</div> : null}
      <section className="feed-section">
        <h2 className="feed-section-title">🎙 Podcasts</h2>
        {podcastsData.error ? <FeedError message={podcastsData.error} /> : <Podcasts podcasts={podcastsData.podcasts} />}
      </section>
      <section className="feed-section">
        <h2 className="feed-section-title">📝 Blogs</h2>
        {blogsData.error ? <FeedError message={blogsData.error} /> : <Blogs blogs={blogsData.blogs} />}
      </section>
      <section className="feed-section">
        <div className="feed-x-head">
          <h2 className="feed-section-title">🐦 X · Latest tweets</h2>
          {!xData.error && xUsers.length > 0 ? (
            <div className="feed-x-controls">
              <select
                id="feed-x-select"
                className="feed-x-select"
                value={xSelHandle}
                style={collapsed ? undefined : { display: 'none' }}
                onChange={(e) => {
                  setHandle(e.target.value);
                  saveXHandle(e.target.value);
                }}
              >
                {xUsers.map((u) => (
                  <option key={u.handle} value={u.handle}>
                    @{u.handle} · {u.name}
                  </option>
                ))}
              </select>
              <button
                type="button"
                id="feed-x-toggle"
                className="feed-x-toggle"
                onClick={() => {
                  const next = !collapsed;
                  setCollapsed(next);
                  saveXCollapsed(next);
                }}
              >
                {collapsed ? 'Expand all' : 'Collapse'}
              </button>
            </div>
          ) : null}
        </div>
        {xData.error ? (
          <FeedError message={xData.error} />
        ) : xUsers.length === 0 ? (
          <div className="feed-empty">No X content</div>
        ) : (
          <>
            <div id="feed-x-collapsed" style={collapsed ? undefined : { display: 'none' }}>
              {xSelUser ? <XUser user={xSelUser} /> : null}
            </div>
            <div id="feed-x-expanded" style={collapsed ? { display: 'none' } : undefined}>
              {xUsers.map((u) => (
                <XUser key={u.handle} user={u} />
              ))}
            </div>
          </>
        )}
      </section>
    </>
  );
}

export function BuildersFeed() {
  const [view, setView] = useState<FeedViewState>({ kind: 'loading' });

  useEffect(() => {
    let cancelled = false;
    void loadBuilderFeeds()
      .then(({ xData, podcastsData, blogsData }) => {
        if (cancelled) return;
        setView({
          kind: 'ready',
          generatedAt: xData.generatedAt || podcastsData.generatedAt || blogsData.generatedAt || '',
          podcastsData,
          blogsData,
          xData,
          xUsers: xData.x || [],
        });
      })
      .catch((e) => {
        if (!cancelled) setView({ kind: 'error', message: errMessage(e, String(e)) });
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (view.kind === 'loading') return <div className="feed-loading">⏳ Loading…</div>;
  if (view.kind === 'error') return <FeedError message={view.message} />;
  return (
    <FeedView
      generatedAt={view.generatedAt}
      podcastsData={view.podcastsData}
      blogsData={view.blogsData}
      xData={view.xData}
      xUsers={view.xUsers}
    />
  );
}

const feedRoots = new WeakMap<Element, Root>();

/** Home-entry slot adapter / tests: mount React feed into the slot. */
export function renderFeed(container: Element) {
  let root = feedRoots.get(container);
  if (!root) {
    root = createRoot(container);
    feedRoots.set(container, root);
  }
  flushSync(() => {
    root.render(<BuildersFeed />);
  });
}
