// ── Feed URLs ──────────────────────────────────────────────────────────────

const FEED_X_URL         = 'https://raw.githubusercontent.com/zarazhangrui/follow-builders/main/feed-x.json';
const FEED_PODCASTS_URL  = 'https://raw.githubusercontent.com/zarazhangrui/follow-builders/main/feed-podcasts.json';
const FEED_BLOGS_URL     = 'https://raw.githubusercontent.com/zarazhangrui/follow-builders/main/feed-blogs.json';

// ── Helpers ────────────────────────────────────────────────────────────────

function escHtml(s) {
  return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

function fmtDate(isoStr) {
  if (!isoStr) return '';
  try {
    return new Date(isoStr).toLocaleString('zh-CN', { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' });
  } catch { return isoStr; }
}

function fmtDateFull(isoStr) {
  if (!isoStr) return '';
  try {
    return new Date(isoStr).toLocaleString('zh-CN', { year: 'numeric', month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' });
  } catch { return isoStr; }
}

// ── Fetch ──────────────────────────────────────────────────────────────────

async function fetchFeed(url) {
  const ts = Date.now();
  const res = await fetch(`${url}?_=${ts}`);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

// ── Render X ──────────────────────────────────────────────────────────────

function renderX(users) {
  if (!users || users.length === 0) {
    return '<div class="feed-empty">暂无 X 内容</div>';
  }
  return users.map(user => {
    const tweets = (user.tweets || []);
    const tweetsHtml = tweets.length === 0
      ? '<div class="feed-tweet-empty">近期无推文</div>'
      : tweets.map(t => `
        <div class="feed-tweet">
          <div class="feed-tweet-text">${escHtml(t.text)}</div>
          <div class="feed-tweet-meta">
            <span class="feed-tweet-ts">${fmtDate(t.createdAt)}</span>
            ${t.likes ? `<span class="feed-tweet-stat">♥ ${t.likes}</span>` : ''}
            ${t.retweets ? `<span class="feed-tweet-stat">🔁 ${t.retweets}</span>` : ''}
            ${t.replies ? `<span class="feed-tweet-stat">💬 ${t.replies}</span>` : ''}
            <a class="feed-tweet-link" href="${escHtml(t.url)}" target="_blank" rel="noopener">原文 ↗</a>
          </div>
        </div>
      `).join('');
    return `
      <div class="feed-person">
        <div class="feed-person-header">
          <span class="feed-person-name">${escHtml(user.name)}</span>
          <a class="feed-person-handle" href="https://x.com/${escHtml(user.handle)}" target="_blank" rel="noopener">@${escHtml(user.handle)} ↗</a>
        </div>
        <div class="feed-person-tweets">${tweetsHtml}</div>
      </div>
    `;
  }).join('');
}

// ── Render Podcasts ────────────────────────────────────────────────────────

function renderPodcasts(podcasts) {
  if (!podcasts || podcasts.length === 0) {
    return '<div class="feed-empty">暂无播客内容</div>';
  }
  return podcasts.map((p, i) => {
    const id = `feed-podcast-${i}`;
    const preview = p.transcript ? p.transcript.slice(0, 300) : '';
    const hasMore = p.transcript && p.transcript.length > 300;
    const transcriptHtml = p.transcript
      ? `
        <div class="feed-podcast-transcript">
          <div class="feed-transcript-preview" id="${id}-preview">${escHtml(preview)}${hasMore ? '…' : ''}</div>
          ${hasMore ? `<div class="feed-transcript-full" id="${id}-full" style="display:none">${escHtml(p.transcript)}</div>
          <button class="feed-transcript-toggle" data-target="${id}">展开全文</button>` : ''}
        </div>
      `
      : '';
    return `
      <div class="feed-podcast">
        <div class="feed-podcast-title"><a href="${escHtml(p.url)}" target="_blank" rel="noopener">${escHtml(p.title)} ↗</a></div>
        <div class="feed-podcast-meta">
          <span class="feed-podcast-show">${escHtml(p.name)}</span>
          <span class="feed-podcast-ts">${fmtDate(p.publishedAt)}</span>
        </div>
        ${transcriptHtml}
      </div>
    `;
  }).join('');
}

// ── Render Blogs ───────────────────────────────────────────────────────────

function renderBlogs(blogs) {
  if (!blogs || blogs.length === 0) {
    return '<div class="feed-empty">暂无博客内容（博客源尚在配置中）</div>';
  }
  return blogs.map(b => `
    <div class="feed-blog">
      <a class="feed-blog-title" href="${escHtml(b.url)}" target="_blank" rel="noopener">${escHtml(b.title)} ↗</a>
      <div class="feed-blog-meta">
        <span>${escHtml(b.author || b.name || '')}</span>
        <span>${fmtDate(b.publishedAt)}</span>
      </div>
    </div>
  `).join('');
}

// ── Main render ────────────────────────────────────────────────────────────

export async function renderFeed(container) {
  container.innerHTML = '<div class="feed-loading">⏳ 加载中…</div>';

  try {
    const [xData, podcastsData, blogsData] = await Promise.all([
      fetchFeed(FEED_X_URL).catch(e => ({ error: e.message })),
      fetchFeed(FEED_PODCASTS_URL).catch(e => ({ error: e.message })),
      fetchFeed(FEED_BLOGS_URL).catch(e => ({ error: e.message })),
    ]);

    const generatedAt = xData.generatedAt || podcastsData.generatedAt || blogsData.generatedAt || '';
    const updatedLine = generatedAt
      ? `<div class="feed-updated">数据更新于 ${fmtDateFull(generatedAt)}</div>`
      : '';

    container.innerHTML = `
      ${updatedLine}
      <section class="feed-section">
        <h2 class="feed-section-title">🐦 X · 最新推文</h2>
        ${xData.error
          ? `<div class="feed-error">加载失败：${escHtml(xData.error)}</div>`
          : renderX(xData.x)
        }
      </section>
      <section class="feed-section">
        <h2 class="feed-section-title">🎙 Podcasts</h2>
        ${podcastsData.error
          ? `<div class="feed-error">加载失败：${escHtml(podcastsData.error)}</div>`
          : renderPodcasts(podcastsData.podcasts)
        }
      </section>
      <section class="feed-section">
        <h2 class="feed-section-title">📝 Blogs</h2>
        ${blogsData.error
          ? `<div class="feed-error">加载失败：${escHtml(blogsData.error)}</div>`
          : renderBlogs(blogsData.blogs)
        }
      </section>
    `;

    // Expand/collapse transcript
    container.querySelectorAll('.feed-transcript-toggle').forEach(btn => {
      btn.addEventListener('click', () => {
        const target = btn.dataset.target;
        const preview = container.querySelector(`#${target}-preview`);
        const full = container.querySelector(`#${target}-full`);
        if (!full) return;
        const expanded = full.style.display !== 'none';
        preview.style.display = expanded ? '' : 'none';
        full.style.display = expanded ? 'none' : '';
        btn.textContent = expanded ? '展开全文' : '收起';
      });
    });

  } catch (e) {
    container.innerHTML = `<div class="feed-error">加载失败：${escHtml(e.message)}</div>`;
  }
}
