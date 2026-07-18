import { searchKnowledge, reindexKnowledge, getReindexStatus } from '../api.js'

let _initialized = false
let _debounceTimer = null
let _pollTimer = null
let _inputFocused = false

const _HIST_KEY = 'gs-history-kb'
const _HIST_MAX = 10

export function initCorpusSearch() {
  if (_initialized) return
  _initialized = true

  const input = document.getElementById('gs-kb-input')
  const rebuildBtn = document.getElementById('gs-kb-rebuild-btn')
  if (!input) return

  input.addEventListener('input', () => {
    clearTimeout(_debounceTimer)
    const q = _normalizeQuery(input.value)
    _updateRebuildUI()
    if (!q) {
      _showHistory()
      return
    }
    _debounceTimer = setTimeout(() => _search(q), 300)
  })

  input.addEventListener('keydown', e => {
    if (e.key === 'Escape') { _close(); input.blur() }
  })

  input.addEventListener('focus', () => {
    _inputFocused = true
    _updateRebuildUI()
    if (!input.value) _showHistory()
  })

  input.addEventListener('blur', () => {
    _inputFocused = false
    setTimeout(() => _updateRebuildUI(), 200)
  })

  if (rebuildBtn) {
    rebuildBtn.addEventListener('click', _startKbRebuild)
  }

  document.addEventListener('click', e => {
    const wrap = document.getElementById('gs-kb-wrap')
    if (wrap && !wrap.contains(e.target)) _close()
  })
}

export function closeCorpusSearch() {
  _close()
}

function _normalizeQuery(raw) {
  return String(raw || '').trim()
}

function _updateRebuildUI() {
  const rebuildBtn = document.getElementById('gs-kb-rebuild-btn')
  if (rebuildBtn && _pollTimer === null) {
    rebuildBtn.style.display = _inputFocused ? 'inline-flex' : 'none'
  }
}

async function _search(q) {
  const dropdown = document.getElementById('gs-kb-dropdown')
  if (!dropdown) return

  _show(dropdown, '<div class="gs-status">Searching…</div>')

  try {
    const data = await searchKnowledge(q, 8)

    if (data.error === 'unavailable') {
      _show(dropdown, '<div class="gs-status">Meilisearch is not running; search unavailable<br><span style="font-size:10px;opacity:.7;">Start external Meilisearch first (default localhost:7700)</span></div>')
      return
    }
    if (data.error === 'not_indexed') {
      _show(dropdown, '<div class="gs-status">Index not built yet. Click ↺ to rebuild.</div>')
      return
    }
    const hits = data.hits || []
    if (hits.length === 0) {
      _show(dropdown, '<div class="gs-status">No related results</div>')
    } else {
      _show(dropdown, _renderKbHits(hits))
    }
  } catch (_) {
    _show(dropdown, '<div class="gs-status">Search error</div>')
  }
}

function _renderKbHits(hits) {
  return hits.map(hit => {
    const title = _esc(hit.title || hit.path || '')
    const repo = _esc((hit.repo || '').split('/').pop())
    const snippet = _getSnippet(hit)
    return `
      <div class="gs-hit gs-hit-kb"
        data-repo="${_esc(hit.repo || '')}"
        data-path="${_esc(hit.path || '')}"
        data-url="${_esc(hit.url || '')}"
        data-title="${_esc(hit.title || hit.path || '')}">
        <div class="gs-hit-title">${title}</div>
        <span class="gs-hit-repo">${repo}</span>
        <div class="gs-hit-snippet">${snippet}</div>
      </div>
    `
  }).join('')
}

async function _startKbRebuild() {
  const btn = document.getElementById('gs-kb-rebuild-btn')
  if (btn) { btn.disabled = true; btn.classList.add('syncing') }

  try {
    const res = await reindexKnowledge()
    if (res.error) { _stopKbRebuild(true, res.error); return }
  } catch (_) {
    _stopKbRebuild(true, 'Request failed')
    return
  }

  _pollTimer = setInterval(_pollKbRebuild, 2000)
}

async function _pollKbRebuild() {
  try {
    const res = await getReindexStatus()
    if (res.status === 'done') {
      _stopKbRebuild(false, res.log)
    } else if (res.status === 'error') {
      _stopKbRebuild(true, res.log || 'Rebuild failed')
    }
  } catch (_) {}
}

function _stopKbRebuild(isError, msg) {
  clearInterval(_pollTimer)
  _pollTimer = null
  const btn = document.getElementById('gs-kb-rebuild-btn')
  if (btn) {
    btn.disabled = false
    btn.classList.remove('syncing')
    btn.title = isError ? `Rebuild failed: ${msg}` : 'Rebuild knowledge index'
  }
  _updateRebuildUI()
  const dropdown = document.getElementById('gs-kb-dropdown')
  if (dropdown) {
    const color = isError ? '#cf222e' : '#1a7f37'
    _show(dropdown, `<div class="gs-status" style="color:${color}">${
      isError ? '❌ Rebuild failed: ' : '✅ Rebuild finished: '
    }${_esc(msg || (isError ? 'Unknown error' : ''))}</div>`)
  }
}

function _getHistory() {
  try {
    return JSON.parse(localStorage.getItem(_HIST_KEY)) || []
  } catch (_) {
    return []
  }
}

function _addHistory(q) {
  const normalized = _normalizeQuery(q)
  if (!normalized) return
  const list = _getHistory().filter(x => x !== normalized)
  list.unshift(normalized)
  localStorage.setItem(_HIST_KEY, JSON.stringify(list.slice(0, _HIST_MAX)))
}

function _removeHistory(q) {
  const list = _getHistory().filter(x => x !== q)
  localStorage.setItem(_HIST_KEY, JSON.stringify(list))
}

function _renderHistory() {
  const list = _getHistory()
  if (!list.length) return ''
  const items = list.map(q => `
    <div class="gs-hist-item" data-q="${_esc(q)}">
      <span class="gs-hist-label">${_esc(q)}</span>
      <button class="gs-hist-remove" data-q="${_esc(q)}" title="Delete">×</button>
    </div>
  `).join('')
  return `<div class="gs-hist-list">${items}</div>`
}

function _showHistory() {
  const dropdown = document.getElementById('gs-kb-dropdown')
  if (!dropdown) return
  const html = _renderHistory()
  if (!html) return
  _show(dropdown, html)
  dropdown.querySelectorAll('.gs-hist-item').forEach(el => {
    el.addEventListener('click', e => {
      if (e.target.classList.contains('gs-hist-remove')) return
      const input = document.getElementById('gs-kb-input')
      if (!input) return
      input.value = el.dataset.q
      input.dispatchEvent(new Event('input', { bubbles: true }))
    })
  })
  dropdown.querySelectorAll('.gs-hist-remove').forEach(btn => {
    btn.addEventListener('click', e => {
      e.stopPropagation()
      _removeHistory(btn.dataset.q)
      _showHistory()
    })
  })
}

function _getSnippet(hit) {
  const formatted = hit._formatted || {}
  const body = formatted.body || hit.body || ''
  const plain = body.replace(/^#{1,6}\s+/gm, '').replace(/[*_`]/g, '')
  return _escKeepEm(plain.slice(0, 120).trim())
}

function _show(dropdown, html) {
  dropdown.innerHTML = html
  dropdown.style.display = 'block'

  dropdown.querySelectorAll('.gs-hit-kb').forEach(el => {
    el.addEventListener('click', () => {
      const repo = el.dataset.repo
      const path = el.dataset.path
      const url = el.dataset.url
      const title = el.dataset.title
      if (repo && path) {
        document.dispatchEvent(new CustomEvent('cta:open-kb-doc', {
          detail: { repo, path, url, title },
        }))
      }
      const input = document.getElementById('gs-kb-input')
      _addHistory(_normalizeQuery(input?.value || ''))
      _close()
    })
  })
}

function _close() {
  const dropdown = document.getElementById('gs-kb-dropdown')
  if (dropdown) dropdown.style.display = 'none'
}

function _esc(str) {
  return String(str).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
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
