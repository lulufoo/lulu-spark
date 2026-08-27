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
    return new Date(isoStr).toLocaleString('en-US', { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' });
  } catch { return isoStr; }
}

function fmtDateFull(isoStr) {
  if (!isoStr) return '';
  try {
    return new Date(isoStr).toLocaleString('en-US', { year: 'numeric', month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' });
  } catch { return isoStr; }
}

// ── Fetch ──────────────────────────────────────────────────────────────────

async function fetchFeed(url) {
  const ts = Date.now();
  const res = await fetch(`${url}?_=${ts}`);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

// ── X State (localStorage) ────────────────────────────────────────────────

function isXCollapsed()     { return localStorage.getItem('cta_x_collapsed') === '1'; }
function getXHandle()       { return localStorage.getItem('cta_x_handle') || ''; }
function saveXCollapsed(v)  { localStorage.setItem('cta_x_collapsed', v ? '1' : '0'); }
function saveXHandle(h)     { localStorage.setItem('cta_x_handle', h); }

// ── Render single X user ──────────────────────────────────────────────────

function renderXUser(user) {
  const tweets = (user.tweets || []);
  const tweetsHtml = tweets.length === 0
    ? '<div class="feed-tweet-empty">No recent tweets</div>'
    : tweets.map(t => `
      <div class="feed-tweet">
        <div class="feed-tweet-text">${escHtml(t.text)}</div>
        <div class="feed-tweet-meta">
          <span class="feed-tweet-ts">${fmtDate(t.createdAt)}</span>
          ${t.likes ? `<span class="feed-tweet-stat">♥ ${t.likes}</span>` : ''}
          ${t.retweets ? `<span class="feed-tweet-stat">🔁 ${t.retweets}</span>` : ''}
          ${t.replies ? `<span class="feed-tweet-stat">💬 ${t.replies}</span>` : ''}
          <a class="feed-tweet-link" href="${escHtml(t.url)}" target="_blank" rel="noopener">Original ↗</a>
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
}

// ── Render Podcasts ────────────────────────────────────────────────────────

function renderPodcasts(podcasts) {
  if (!podcasts || podcasts.length === 0) {
    return '<div class="feed-empty">No podcasts</div>';
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
          <button class="feed-transcript-toggle" data-target="${id}">Expand</button>` : ''}
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
    return '<div class="feed-empty">No blogs yet (blog sources still being configured)</div>';
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
  container.innerHTML = '<div class="feed-loading">⏳ Loading…</div>';

  try {
    const [xData, podcastsData, blogsData] = await Promise.all([
      fetchFeed(FEED_X_URL).catch(e => ({ error: e.message })),
      fetchFeed(FEED_PODCASTS_URL).catch(e => ({ error: e.message })),
      fetchFeed(FEED_BLOGS_URL).catch(e => ({ error: e.message })),
    ]);

    const generatedAt = xData.generatedAt || podcastsData.generatedAt || blogsData.generatedAt || '';
    const updatedLine = generatedAt
      ? `<div class="feed-updated">Updated ${fmtDateFull(generatedAt)}</div>`
      : '';

    // ── X section state ────────────────────────────────────────────────────
    const xUsers      = xData.x || [];
    const xCollapsed  = isXCollapsed();
    const savedHandle = getXHandle();
    const xSelHandle  = (savedHandle && xUsers.find(u => u.handle === savedHandle))
      ? savedHandle : (xUsers[0] ? xUsers[0].handle : '');
    const xSelUser    = xUsers.find(u => u.handle === xSelHandle) || xUsers[0];
    const xOptions    = xUsers.map(u =>
      `<option value="${escHtml(u.handle)}"${u.handle === xSelHandle ? ' selected' : ''}>@${escHtml(u.handle)} · ${escHtml(u.name)}</option>`
    ).join('');

    container.innerHTML = `
      ${updatedLine}
      <section class="feed-section">
        <h2 class="feed-section-title">🎙 Podcasts</h2>
        ${podcastsData.error
          ? `<div class="feed-error">Failed to load: ${escHtml(podcastsData.error)}</div>`
          : renderPodcasts(podcastsData.podcasts)
        }
      </section>
      <section class="feed-section">
        <h2 class="feed-section-title">📝 Blogs</h2>
        ${blogsData.error
          ? `<div class="feed-error">Failed to load: ${escHtml(blogsData.error)}</div>`
          : renderBlogs(blogsData.blogs)
        }
      </section>
      <section class="feed-section">
        <div class="feed-x-head">
          <h2 class="feed-section-title">🐦 X · Latest tweets</h2>
          ${!xData.error && xUsers.length > 0 ? `
          <div class="feed-x-controls">
            <select id="feed-x-select" class="feed-x-select"${xCollapsed ? '' : ' style="display:none"'}>${xOptions}</select>
            <button id="feed-x-toggle" class="feed-x-toggle">${xCollapsed ? 'Expand all' : 'Collapse'}</button>
          </div>` : ''}
        </div>
        ${xData.error
          ? `<div class="feed-error">Failed to load: ${escHtml(xData.error)}</div>`
          : xUsers.length === 0
            ? '<div class="feed-empty">No X content</div>'
            : `
          <div id="feed-x-collapsed"${xCollapsed ? '' : ' style="display:none"'}>${xSelUser ? renderXUser(xSelUser) : ''}</div>
          <div id="feed-x-expanded"${xCollapsed ? ' style="display:none"' : ''}>${xUsers.map(u => renderXUser(u)).join('')}</div>
        `}
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
        btn.textContent = expanded ? 'Expand' : 'Collapse';
      });
    });

    // X toggle and user select
    const xToggleBtn   = container.querySelector('#feed-x-toggle');
    const xSelectEl    = container.querySelector('#feed-x-select');
    const xCollapsedEl = container.querySelector('#feed-x-collapsed');
    const xExpandedEl  = container.querySelector('#feed-x-expanded');

    if (xToggleBtn) {
      xToggleBtn.addEventListener('click', () => {
        const nowCollapsed = xCollapsedEl && xCollapsedEl.style.display !== 'none';
        if (nowCollapsed) {
          // collapse → expand
          if (xCollapsedEl) xCollapsedEl.style.display = 'none';
          if (xSelectEl)    xSelectEl.style.display    = 'none';
          if (xExpandedEl)  xExpandedEl.style.display  = '';
          xToggleBtn.textContent = 'Collapse';
          saveXCollapsed(false);
        } else {
          // expand → collapse
          if (xCollapsedEl) xCollapsedEl.style.display = '';
          if (xSelectEl)    xSelectEl.style.display    = '';
          if (xExpandedEl)  xExpandedEl.style.display  = 'none';
          xToggleBtn.textContent = 'Expand all';
          saveXCollapsed(true);
        }
      });
    }

    if (xSelectEl && xCollapsedEl) {
      xSelectEl.addEventListener('change', () => {
        const handle = xSelectEl.value;
        saveXHandle(handle);
        const user = xUsers.find(u => u.handle === handle);
        if (user) xCollapsedEl.innerHTML = renderXUser(user);
      });
    }

  } catch (e) {
    container.innerHTML = `<div class="feed-error">Failed to load: ${escHtml(e.message)}</div>`;
  }
}
