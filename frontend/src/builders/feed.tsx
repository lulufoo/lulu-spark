// @ts-nocheck — ported from JS; feed payloads stay unchecked like checkJs:false.
import { renderToHtml } from '../island.ts';

// ── Feed URLs ──────────────────────────────────────────────────────────────

const FEED_X_URL = 'https://raw.githubusercontent.com/zarazhangrui/follow-builders/main/feed-x.json';
const FEED_PODCASTS_URL =
  'https://raw.githubusercontent.com/zarazhangrui/follow-builders/main/feed-podcasts.json';
const FEED_BLOGS_URL = 'https://raw.githubusercontent.com/zarazhangrui/follow-builders/main/feed-blogs.json';

// ── Helpers ────────────────────────────────────────────────────────────────

function fmtDate(isoStr) {
  if (!isoStr) return '';
  try {
    return new Date(isoStr).toLocaleString('en-US', {
      month: 'numeric',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return isoStr;
  }
}

function fmtDateFull(isoStr) {
  if (!isoStr) return '';
  try {
    return new Date(isoStr).toLocaleString('en-US', {
      year: 'numeric',
      month: 'numeric',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return isoStr;
  }
}

// ── Fetch ──────────────────────────────────────────────────────────────────

async function fetchFeed(url) {
  const ts = Date.now();
  const res = await fetch(`${url}?_=${ts}`);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

// ── X State (localStorage) ────────────────────────────────────────────────

function isXCollapsed() {
  return localStorage.getItem('cta_x_collapsed') === '1';
}
function getXHandle() {
  return localStorage.getItem('cta_x_handle') || '';
}
function saveXCollapsed(v) {
  localStorage.setItem('cta_x_collapsed', v ? '1' : '0');
}
function saveXHandle(h) {
  localStorage.setItem('cta_x_handle', h);
}

// ── JSX pieces ─────────────────────────────────────────────────────────────

function XUser({ user }) {
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

function Podcasts({ podcasts }) {
  if (!podcasts || podcasts.length === 0) {
    return <div className="feed-empty">No podcasts</div>;
  }
  return (
    <>
      {podcasts.map((p, i) => {
        const id = `feed-podcast-${i}`;
        const preview = p.transcript ? p.transcript.slice(0, 300) : '';
        const hasMore = p.transcript && p.transcript.length > 300;
        return (
          <div className="feed-podcast" key={p.url || id}>
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
                <div className="feed-transcript-preview" id={`${id}-preview`}>
                  {preview}
                  {hasMore ? '…' : ''}
                </div>
                {hasMore ? (
                  <>
                    <div className="feed-transcript-full" id={`${id}-full`} style={{ display: 'none' }}>
                      {p.transcript}
                    </div>
                    <button type="button" className="feed-transcript-toggle" data-target={id}>
                      Expand
                    </button>
                  </>
                ) : null}
              </div>
            ) : null}
          </div>
        );
      })}
    </>
  );
}

function Blogs({ blogs }) {
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

function FeedError({ message }) {
  return <div className="feed-error">Failed to load: {message}</div>;
}

function FeedView({ generatedAt, podcastsData, blogsData, xData, xUsers, xCollapsed, xSelHandle, xSelUser }) {
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
                defaultValue={xSelHandle}
                style={xCollapsed ? undefined : { display: 'none' }}
              >
                {xUsers.map((u) => (
                  <option key={u.handle} value={u.handle}>
                    @{u.handle} · {u.name}
                  </option>
                ))}
              </select>
              <button type="button" id="feed-x-toggle" className="feed-x-toggle">
                {xCollapsed ? 'Expand all' : 'Collapse'}
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
            <div id="feed-x-collapsed" style={xCollapsed ? undefined : { display: 'none' }}>
              {xSelUser ? <XUser user={xSelUser} /> : null}
            </div>
            <div id="feed-x-expanded" style={xCollapsed ? { display: 'none' } : undefined}>
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

function wireFeed(container, xUsers) {
  container.querySelectorAll('.feed-transcript-toggle').forEach((btn) => {
    btn.addEventListener('click', () => {
      const target = btn.dataset.target;
      const preview = container.querySelector(`#${target}-preview`);
      const full = container.querySelector(`#${target}-full`);
      if (!full) return;
      const expanded = full.style.display !== 'none';
      preview.style.display = expanded ? '' : 'none';
      full.style.display = expanded ? 'none' : '';
      btn.textContent = expanded ? 'Expand' : 'Collapse';
    });
  });

  const xToggleBtn = container.querySelector('#feed-x-toggle');
  const xSelectEl = container.querySelector('#feed-x-select');
  const xCollapsedEl = container.querySelector('#feed-x-collapsed');
  const xExpandedEl = container.querySelector('#feed-x-expanded');

  if (xToggleBtn) {
    xToggleBtn.addEventListener('click', () => {
      const nowCollapsed = xCollapsedEl && xCollapsedEl.style.display !== 'none';
      if (nowCollapsed) {
        if (xCollapsedEl) xCollapsedEl.style.display = 'none';
        if (xSelectEl) xSelectEl.style.display = 'none';
        if (xExpandedEl) xExpandedEl.style.display = '';
        xToggleBtn.textContent = 'Collapse';
        saveXCollapsed(false);
      } else {
        if (xCollapsedEl) xCollapsedEl.style.display = '';
        if (xSelectEl) xSelectEl.style.display = '';
        if (xExpandedEl) xExpandedEl.style.display = 'none';
        xToggleBtn.textContent = 'Expand all';
        saveXCollapsed(true);
      }
    });
  }

  if (xSelectEl && xCollapsedEl) {
    xSelectEl.addEventListener('change', () => {
      const handle = xSelectEl.value;
      saveXHandle(handle);
      const user = xUsers.find((u) => u.handle === handle);
      if (user) xCollapsedEl.innerHTML = renderToHtml(<XUser user={user} />);
    });
  }
}

// ── Main render ────────────────────────────────────────────────────────────

export async function renderFeed(container) {
  container.innerHTML = renderToHtml(<div className="feed-loading">⏳ Loading…</div>);

  try {
    const [xData, podcastsData, blogsData] = await Promise.all([
      fetchFeed(FEED_X_URL).catch((e) => ({ error: e.message })),
      fetchFeed(FEED_PODCASTS_URL).catch((e) => ({ error: e.message })),
      fetchFeed(FEED_BLOGS_URL).catch((e) => ({ error: e.message })),
    ]);

    const generatedAt = xData.generatedAt || podcastsData.generatedAt || blogsData.generatedAt || '';
    const xUsers = xData.x || [];
    const xCollapsed = isXCollapsed();
    const savedHandle = getXHandle();
    const xSelHandle =
      savedHandle && xUsers.find((u) => u.handle === savedHandle)
        ? savedHandle
        : xUsers[0]
          ? xUsers[0].handle
          : '';
    const xSelUser = xUsers.find((u) => u.handle === xSelHandle) || xUsers[0];

    container.innerHTML = renderToHtml(
      <FeedView
        generatedAt={generatedAt}
        podcastsData={podcastsData}
        blogsData={blogsData}
        xData={xData}
        xUsers={xUsers}
        xCollapsed={xCollapsed}
        xSelHandle={xSelHandle}
        xSelUser={xSelUser}
      />,
    );

    wireFeed(container, xUsers);
  } catch (e) {
    container.innerHTML = renderToHtml(<FeedError message={e.message} />);
  }
}
