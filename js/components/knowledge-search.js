import { searchKnowledge, reindexKnowledge, getReindexStatus } from '../api.js'

// ── State ─────────────────────────────────────────────────────────────────────
// Possible states: idle | loading | showing | empty | syncing | unavailable
let _container = null
let _state = 'idle'
let _pollTimer = null
let _debounceTimer = null

// ── Mount ─────────────────────────────────────────────────────────────────────

export function mountKnowledgeSearch(container) {
  if (!container) return
  _container = container
  _container.innerHTML = _buildSkeleton()
  // sync arrow with current collapse state (persists across re-mounts)
  const toggleBtn = _container.querySelector('.ks-toggle')
  if (toggleBtn) toggleBtn.textContent = _container.classList.contains('ks-collapsed') ? '‹' : '›'
  _bindEvents()
}

// ── Trigger (called when entry opens) ────────────────────────────────────────

export function triggerKnowledgeSearch(entry) {
  if (!_container) return
  const q = _buildQuery(entry)
  const input = _container.querySelector('.ks-input')
  if (input) input.value = q
  _search(q)
}

// ── Query builder ─────────────────────────────────────────────────────────────

function _buildQuery(entry) {
  const parts = []
  if (entry.title && entry.title.trim()) parts.push(entry.title.trim())
  // Extract slug: last path segment, strip timestamp prefix (14-digit or 12-digit)
  const cp = entry.common_path || ''
  const filename = cp.split('/').pop() || ''
  const slug = filename.replace(/\.md$/, '').replace(/^\d{8,14}-/, '')
  if (slug && slug !== entry.title) parts.push(slug)
  return parts.join(' ')
}

// ── HTML skeleton ─────────────────────────────────────────────────────────────

function _buildSkeleton() {
  return `
    <button class="ks-toggle" title="展开知识面板">‹</button>
    <div class="ks-inner">
      <div class="ks-panel-title">相关知识<button class="ks-refresh-btn" title="同步知识库">↺</button></div>
      <div class="ks-search-bar">
        <input class="ks-input" type="text" placeholder="搜索知识库…" autocomplete="off" />
      </div>
      <div class="ks-sync-bar" style="display:none">⟳ 正在同步知识库…</div>
      <div class="ks-results">
        <div class="ks-status-msg"></div>
      </div>
    </div>
  `
}

// ── Event binding ─────────────────────────────────────────────────────────────

function _bindEvents() {
  const input = _container.querySelector('.ks-input')
  if (input) {
    input.addEventListener('input', () => {
      clearTimeout(_debounceTimer)
      _debounceTimer = setTimeout(() => _search(input.value.trim()), 300)
    })
  }

  const refreshBtn = _container.querySelector('.ks-refresh-btn')
  if (refreshBtn) {
    refreshBtn.addEventListener('click', _startSync)
  }

  const toggleBtn = _container.querySelector('.ks-toggle')
  if (toggleBtn) {
    toggleBtn.addEventListener('click', _togglePanel)
  }
}

// ── Toggle collapse ────────────────────────────────────────────────────────────

function _togglePanel() {
  if (!_container) return
  const btn = _container.querySelector('.ks-toggle')
  const collapsed = _container.classList.toggle('ks-collapsed')
  if (btn) btn.textContent = collapsed ? '‹' : '›'
}

// ── Search ────────────────────────────────────────────────────────────────────

async function _search(q) {
  if (!q) {
    _renderStatusMsg('输入关键词开始搜索')
    return
  }

  _setState('loading')
  _renderStatusMsg('正在搜索…')

  try {
    const data = await searchKnowledge(q, 10)

    if (data.error === 'unavailable') {
      _setState('unavailable')
      _container.classList.add('ks-unavailable')
      return
    }

    // Restore visibility if it was previously hidden
    _container.classList.remove('ks-unavailable')

    const hits = data.hits || []
    if (hits.length === 0) {
      _setState('empty')
      _renderStatusMsg('暂无相关知识<br><span style="font-size:10px;color:#aaa">换一个关键词试试？</span>')
    } else {
      _setState('showing')
      _renderHits(hits)
    }
  } catch (_) {
    _renderStatusMsg('搜索出错')
  }
}

// ── Sync ──────────────────────────────────────────────────────────────────────

async function _startSync() {
  const syncBar = _container.querySelector('.ks-sync-bar')
  const refreshBtn = _container.querySelector('.ks-refresh-btn')
  if (refreshBtn) refreshBtn.disabled = true
  if (syncBar) {
    syncBar.className = 'ks-sync-bar'
    syncBar.textContent = '⟳ 正在同步知识库…'
    syncBar.style.display = 'block'
  }

  try {
    const res = await reindexKnowledge()
    if (res.error) {
      _showSyncError(res.error)
      return
    }
  } catch (_) {
    _showSyncError('请求失败')
    return
  }

  _setState('syncing')
  _pollTimer = setInterval(_pollSync, 2000)
}

async function _pollSync() {
  try {
    const res = await getReindexStatus()
    if (res.status === 'done') {
      _stopSync(false)
      // Re-run current search
      const input = _container.querySelector('.ks-input')
      if (input && input.value.trim()) _search(input.value.trim())
    } else if (res.status === 'error') {
      _stopSync(true, res.log || '同步失败')
    }
  } catch (_) {}
}

function _stopSync(isError, msg) {
  clearInterval(_pollTimer)
  _pollTimer = null

  const refreshBtn = _container.querySelector('.ks-refresh-btn')
  if (refreshBtn) refreshBtn.disabled = false

  const syncBar = _container.querySelector('.ks-sync-bar')
  if (isError) {
    if (syncBar) {
      syncBar.className = 'ks-sync-bar ks-error'
      syncBar.textContent = `⚠ 同步失败：${msg}`
    }
  } else {
    if (syncBar) syncBar.style.display = 'none'
  }
}

// ── Render hits ───────────────────────────────────────────────────────────────

function _renderHits(hits) {
  const resultsEl = _container.querySelector('.ks-results')
  if (!resultsEl) return

  const cards = hits.map(hit => {
    const title = _esc(hit.title || hit.path || '')
    const repo = _esc((hit.repo || '').split('/').pop())
    const url = _esc(hit.url || '#')
    // Meilisearch returns highlight inside _formatted
    const snippet = _getSnippet(hit)
    return `
      <a class="ks-hit" href="${url}" target="_blank" rel="noopener noreferrer">
        <div class="ks-hit-title">${title}</div>
        <span class="ks-hit-repo">${repo}</span>
        <div class="ks-hit-snippet">${snippet}</div>
      </a>
    `
  }).join('')

  resultsEl.innerHTML = cards
}

function _getSnippet(hit) {
  // _formatted contains <em> highlighted tokens from Meilisearch
  const formatted = hit._formatted || {}
  const body = formatted.body || hit.body || ''
  // Truncate to first 160 chars, stripping any remaining markdown syntax
  const plain = body.replace(/^#{1,6}\s+/gm, '').replace(/[*_`]/g, '')
  const snippet = plain.slice(0, 160).trim()
  return _escKeepEm(snippet)
}

// ── State helpers ─────────────────────────────────────────────────────────────

function _setState(s) {
  _state = s
}

function _renderStatusMsg(html) {
  const resultsEl = _container.querySelector('.ks-results')
  if (resultsEl) {
    resultsEl.innerHTML = `<div class="ks-status-msg">${html}</div>`
  }
}

// ── Escape helpers ────────────────────────────────────────────────────────────

function _esc(str) {
  return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

// Escape HTML but preserve <em> and </em> from Meilisearch highlights (trusted source)
function _escKeepEm(str) {
  return str
    .replace(/&/g, '&amp;')
    .replace(/<em>/g, '\x00EM\x00')
    .replace(/<\/em>/g, '\x00_EM\x00')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/\x00EM\x00/g, '<em>')
    .replace(/\x00_EM\x00/g, '</em>')
}
