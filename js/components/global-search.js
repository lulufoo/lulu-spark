import { searchKnowledge } from '../api.js'

// ── State ─────────────────────────────────────────────────────────────────────
let _debounceTimer = null
let _open = false

// ── Init ──────────────────────────────────────────────────────────────────────

export function initGlobalSearch() {
  const input = document.getElementById('gs-input')
  const dropdown = document.getElementById('gs-dropdown')
  if (!input || !dropdown) return

  input.addEventListener('input', () => {
    clearTimeout(_debounceTimer)
    const q = input.value.trim()
    if (!q) { _close(); return }
    _debounceTimer = setTimeout(() => _search(q), 300)
  })

  input.addEventListener('keydown', e => {
    if (e.key === 'Escape') { _close(); input.blur() }
  })

  // Close on outside click
  document.addEventListener('click', e => {
    const wrap = document.getElementById('gs-wrap')
    if (wrap && !wrap.contains(e.target)) _close()
  })
}

// ── Search ────────────────────────────────────────────────────────────────────

async function _search(q) {
  const dropdown = document.getElementById('gs-dropdown')
  if (!dropdown) return

  _show(dropdown, '<div class="gs-status">正在搜索…</div>')

  try {
    const data = await searchKnowledge(q, 8)
    if (data.error === 'unavailable') {
      _show(dropdown, '<div class="gs-status">知识库不可用</div>')
      return
    }
    const hits = data.hits || []
    if (hits.length === 0) {
      _show(dropdown, '<div class="gs-status">暂无相关知识</div>')
    } else {
      _show(dropdown, _renderHits(hits))
    }
  } catch (_) {
    _show(dropdown, '<div class="gs-status">搜索出错</div>')
  }
}

// ── Render ────────────────────────────────────────────────────────────────────

function _renderHits(hits) {
  return hits.map(hit => {
    const title = _esc(hit.title || hit.path || '')
    const repo = _esc((hit.repo || '').split('/').pop())
    const url = _esc(hit.url || '#')
    const snippet = _getSnippet(hit)
    return `
      <a class="gs-hit" href="${url}" target="_blank" rel="noopener noreferrer">
        <div class="gs-hit-title">${title}</div>
        <span class="gs-hit-repo">${repo}</span>
        <div class="gs-hit-snippet">${snippet}</div>
      </a>
    `
  }).join('')
}

function _getSnippet(hit) {
  const formatted = hit._formatted || {}
  const body = formatted.body || hit.body || ''
  const plain = body.replace(/^#{1,6}\s+/gm, '').replace(/[*_`]/g, '')
  return _escKeepEm(plain.slice(0, 120).trim())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function _show(dropdown, html) {
  dropdown.innerHTML = html
  dropdown.style.display = 'block'
  _open = true
}

function _close() {
  const dropdown = document.getElementById('gs-dropdown')
  if (dropdown) dropdown.style.display = 'none'
  _open = false
}

function _esc(str) {
  return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

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
