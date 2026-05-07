import { searchKnowledge, searchWorkbench, reindexWorkbench, getReindexWorkbenchStatus } from '../api.js'

// ── State ─────────────────────────────────────────────────────────────────────
let _debounceTimer = null
let _pollTimer = null

// ── Init ──────────────────────────────────────────────────────────────────────

export function initGlobalSearch() {
  const input = document.getElementById('gs-input')
  const rebuildBtn = document.getElementById('gs-rebuild-btn')
  if (!input) return

  input.addEventListener('input', () => {
    clearTimeout(_debounceTimer)
    const raw = input.value
    const { mode, q } = _detectMode(raw)
    _updateModeUI(mode)
    if (!q) { _close(); return }
    _debounceTimer = setTimeout(() => _search(mode, q), 300)
  })

  input.addEventListener('keydown', e => {
    if (e.key === 'Escape') { _close(); input.blur() }
  })

  if (rebuildBtn) {
    rebuildBtn.addEventListener('click', _startWbRebuild)
  }

  document.addEventListener('click', e => {
    const wrap = document.getElementById('gs-wrap')
    if (wrap && !wrap.contains(e.target)) _close()
  })
}

// ── Mode detection ────────────────────────────────────────────────────────────

function _detectMode(raw) {
  if (raw.startsWith('#')) {
    return { mode: 'wb', q: raw.slice(1).trim() }
  }
  return { mode: 'kb', q: raw.trim() }
}

function _updateModeUI(mode) {
  const pill = document.getElementById('gs-mode-pill')
  const rebuildBtn = document.getElementById('gs-rebuild-btn')
  const isWb = mode === 'wb'
  if (pill) pill.style.display = isWb ? 'inline-flex' : 'none'
  if (rebuildBtn && _pollTimer === null) {
    rebuildBtn.style.display = isWb ? 'inline-flex' : 'none'
  }
}

// ── Search ────────────────────────────────────────────────────────────────────

async function _search(mode, q) {
  const dropdown = document.getElementById('gs-dropdown')
  if (!dropdown) return

  _show(dropdown, '<div class="gs-status">正在搜索…</div>')

  try {
    const data = mode === 'wb'
      ? await searchWorkbench(q, 8)
      : await searchKnowledge(q, 8)

    if (data.error === 'unavailable') {
      _show(dropdown, '<div class="gs-status">知识库不可用</div>')
      return
    }
    const hits = data.hits || []
    if (hits.length === 0) {
      _show(dropdown, '<div class="gs-status">暂无相关结果</div>')
    } else {
      _show(dropdown, mode === 'wb' ? _renderWbHits(hits) : _renderKbHits(hits))
    }
  } catch (_) {
    _show(dropdown, '<div class="gs-status">搜索出错</div>')
  }
}

// ── Render knowledge hits ─────────────────────────────────────────────────────

function _renderKbHits(hits) {
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

// ── Render workbench hits ─────────────────────────────────────────────────────

const _LAYER_LABEL = {
  raw: '原文', distilled: '精炼', digest: '摘要', diagnose: '诊断',
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

// ── Workbench rebuild ─────────────────────────────────────────────────────────

async function _startWbRebuild() {
  const btn = document.getElementById('gs-rebuild-btn')
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
      _stopWbRebuild(false)
    } else if (res.status === 'error') {
      _stopWbRebuild(true, res.log || '重建失败')
    }
  } catch (_) {}
}

function _stopWbRebuild(isError, msg) {
  clearInterval(_pollTimer)
  _pollTimer = null
  const btn = document.getElementById('gs-rebuild-btn')
  if (btn) {
    btn.disabled = false
    btn.classList.remove('syncing')
    if (isError) btn.title = `重建失败：${msg}`
    else btn.title = '重建 Workbench 索引'
  }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function _getSnippet(hit) {
  const formatted = hit._formatted || {}
  const body = formatted.body || hit.body || ''
  const plain = body.replace(/^#{1,6}\s+/gm, '').replace(/[*_`]/g, '')
  return _escKeepEm(plain.slice(0, 120).trim())
}

function _show(dropdown, html) {
  dropdown.innerHTML = html
  dropdown.style.display = 'block'

  // Attach click handlers for workbench hits
  dropdown.querySelectorAll('.gs-hit-wb').forEach(el => {
    el.addEventListener('click', () => {
      const cp = el.dataset.commonPath
      if (cp) {
        document.dispatchEvent(new CustomEvent('cta:open-entry', { detail: { common_path: cp } }))
      }
      _close()
    })
  })
}

function _close() {
  const dropdown = document.getElementById('gs-dropdown')
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
