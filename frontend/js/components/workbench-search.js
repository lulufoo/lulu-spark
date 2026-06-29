import { searchWorkbench, reindexWorkbench, getReindexWorkbenchStatus } from '../api.js'

let _initialized = false
let _debounceTimer = null
let _pollTimer = null
let _inputFocused = false

const _HIST_KEY = 'gs-history-wb'
const _HIST_MAX = 10

const _LAYER_LABEL = {
  raw: '原文', distilled: '精炼', digest: '摘要', diagnose: '诊断',
}

export function initWorkbenchSearch() {
  if (_initialized) return
  _initialized = true

  const input = document.getElementById('gs-wb-input')
  const rebuildBtn = document.getElementById('gs-wb-rebuild-btn')
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
    rebuildBtn.addEventListener('click', _startWbRebuild)
  }

  document.addEventListener('click', e => {
    const wrap = document.getElementById('gs-wb-wrap')
    if (wrap && !wrap.contains(e.target)) _close()
  })
}

export function closeWorkbenchSearch() {
  _close()
}

function _normalizeQuery(raw) {
  const trimmed = String(raw || '').trim()
  return trimmed.startsWith('#') ? trimmed.slice(1).trim() : trimmed
}

function _updateRebuildUI() {
  const rebuildBtn = document.getElementById('gs-wb-rebuild-btn')
  if (rebuildBtn && _pollTimer === null) {
    rebuildBtn.style.display = _inputFocused ? 'inline-flex' : 'none'
  }
}

async function _search(q) {
  const dropdown = document.getElementById('gs-wb-dropdown')
  if (!dropdown) return

  _show(dropdown, '<div class="gs-status">正在搜索…</div>')

  try {
    const data = await searchWorkbench(q, 8)

    if (data.error === 'unavailable') {
      _show(dropdown, '<div class="gs-status">Meilisearch 未运行，搜索不可用<br><span style="font-size:10px;opacity:.7;">请先启动外置 Meilisearch（默认 localhost:7700）</span></div>')
      return
    }
    if (data.error === 'not_indexed') {
      _show(dropdown, '<div class="gs-status">索引尚未建立，请点击 ↺ 重建索引</div>')
      return
    }
    const hits = data.hits || []
    if (hits.length === 0) {
      _show(dropdown, '<div class="gs-status">暂无相关结果</div>')
    } else {
      _show(dropdown, _renderWbHits(hits))
    }
  } catch (_) {
    _show(dropdown, '<div class="gs-status">搜索出错</div>')
  }
}

function _renderWbHits(hits) {
  return hits.map(hit => {
    const title = _esc(hit.title || hit.common_path || '')
    const layer = hit.layer || ''
    const topic = _esc(hit.topic || '')
    const cp = _esc(hit.common_path || '')
    const snippet = _getSnippet(hit)
    return `
      <div class="gs-hit gs-hit-wb" data-common-path="${cp}" data-layer="${_esc(layer)}">
        <div class="gs-hit-title">${title}</div>
        <span class="gs-hit-layer gs-layer-${_esc(layer)}">${_esc(_LAYER_LABEL[layer] || layer)}</span>
        <span class="gs-hit-repo">${topic}</span>
        <div class="gs-hit-snippet">${snippet}</div>
      </div>
    `
  }).join('')
}

async function _startWbRebuild() {
  const btn = document.getElementById('gs-wb-rebuild-btn')
  if (btn) { btn.disabled = true; btn.classList.add('syncing') }

  try {
    const res = await reindexWorkbench()
    if (res.error) { _stopWbRebuild(true, res.error); return }
  } catch (_) {
    _stopWbRebuild(true, '请求失败')
    return
  }

  _pollTimer = setInterval(_pollWbRebuild, 2000)
}

async function _pollWbRebuild() {
  try {
    const res = await getReindexWorkbenchStatus()
    if (res.status === 'done') {
      _stopWbRebuild(false, res.log)
    } else if (res.status === 'error') {
      _stopWbRebuild(true, res.log || '重建失败')
    }
  } catch (_) {}
}

function _stopWbRebuild(isError, msg) {
  clearInterval(_pollTimer)
  _pollTimer = null
  const btn = document.getElementById('gs-wb-rebuild-btn')
  if (btn) {
    btn.disabled = false
    btn.classList.remove('syncing')
    btn.title = isError ? `重建失败：${msg}` : '重建 Workbench 索引'
  }
  _updateRebuildUI()
  const dropdown = document.getElementById('gs-wb-dropdown')
  if (dropdown) {
    const color = isError ? '#cf222e' : '#1a7f37'
    _show(dropdown, `<div class="gs-status" style="color:${color}">${
      isError ? '❌ 重建失败：' : '✅ 重建完成：'
    }${_esc(msg || (isError ? '未知错误' : ''))}</div>`)
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
      <button class="gs-hist-remove" data-q="${_esc(q)}" title="删除">×</button>
    </div>
  `).join('')
  return `<div class="gs-hist-list">${items}</div>`
}

function _showHistory() {
  const dropdown = document.getElementById('gs-wb-dropdown')
  if (!dropdown) return
  const html = _renderHistory()
  if (!html) return
  _show(dropdown, html)
  dropdown.querySelectorAll('.gs-hist-item').forEach(el => {
    el.addEventListener('click', e => {
      if (e.target.classList.contains('gs-hist-remove')) return
      const input = document.getElementById('gs-wb-input')
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

  dropdown.querySelectorAll('.gs-hit-wb').forEach(el => {
    el.addEventListener('click', () => {
      const cp = el.dataset.commonPath
      if (cp) {
        document.dispatchEvent(new CustomEvent('cta:open-entry', { detail: { common_path: cp } }))
      }
      const input = document.getElementById('gs-wb-input')
      _addHistory(_normalizeQuery(input?.value || ''))
      _close()
    })
  })
}

function _close() {
  const dropdown = document.getElementById('gs-wb-dropdown')
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
