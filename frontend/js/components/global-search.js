import { searchKnowledge, searchWorkbench, reindexWorkbench, getReindexWorkbenchStatus, reindexKnowledge, getReindexStatus } from '../api.js'

// ── State ─────────────────────────────────────────────────────────────────────
let _debounceTimer = null
let _pollTimer = null
let _kbPollTimer = null
let _inputFocused = false

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
    if (!q) {
      if (mode === 'wb') _showHistory('wb')
      else _close()
      return
    }
    _debounceTimer = setTimeout(() => _search(mode, q), 300)
  })

  input.addEventListener('keydown', e => {
    if (e.key === 'Escape') { _close(); input.blur() }
  })

  input.addEventListener('focus', () => {
    _inputFocused = true
    const { mode } = _detectMode(input.value)
    _updateModeUI(mode)
    if (!input.value) _showHistory('kb')
  })

  input.addEventListener('blur', () => {
    _inputFocused = false
    // delay so button clicks register before hiding
    setTimeout(() => _updateModeUI(_detectMode(input.value).mode), 200)
  })

  if (rebuildBtn) {
    rebuildBtn.addEventListener('click', _startWbRebuild)
  }

  const kbRebuildBtn = document.getElementById('gs-kb-rebuild-btn')
  if (kbRebuildBtn) {
    kbRebuildBtn.addEventListener('click', _startKbRebuild)
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
  const kbRebuildBtn = document.getElementById('gs-kb-rebuild-btn')
  const isWb = mode === 'wb'
  if (pill) pill.style.display = isWb ? 'inline-flex' : 'none'
  if (rebuildBtn && _pollTimer === null) {
    rebuildBtn.style.display = isWb ? 'inline-flex' : 'none'
  }
  if (kbRebuildBtn && _kbPollTimer === null) {
    kbRebuildBtn.style.display = (!isWb && _inputFocused) ? 'inline-flex' : 'none'
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
      _stopWbRebuild(false, res.log)
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
    btn.title = isError ? `重建失败：${msg}` : '重建 Workbench 索引'
  }
  const dropdown = document.getElementById('gs-dropdown')
  if (dropdown) {
    const color = isError ? '#cf222e' : '#1a7f37'
    _show(dropdown, `<div class="gs-status" style="color:${color}">${
      isError ? '❌ 重建失败：' : '✅ 重建完成：'
    }${_esc(msg || (isError ? '未知错误' : ''))}</div>`)
  }
}

// ── Knowledge base rebuild ───────────────────────────────────────────────────

async function _startKbRebuild() {
  const btn = document.getElementById('gs-kb-rebuild-btn')
  if (btn) { btn.disabled = true; btn.classList.add('syncing') }

  try {
    const res = await reindexKnowledge()
    if (res.error) { _stopKbRebuild(true, res.error); return }
  } catch (_) {
    _stopKbRebuild(true, '请求失败')
    return
  }

  _kbPollTimer = setInterval(_pollKbRebuild, 2000)
}

async function _pollKbRebuild() {
  try {
    const res = await getReindexStatus()
    if (res.status === 'done') {
      _stopKbRebuild(false, res.log)
    } else if (res.status === 'error') {
      _stopKbRebuild(true, res.log || '重建失败')
    }
  } catch (_) {}
}

function _stopKbRebuild(isError, msg) {
  clearInterval(_kbPollTimer)
  _kbPollTimer = null
  const btn = document.getElementById('gs-kb-rebuild-btn')
  if (btn) {
    btn.disabled = false
    btn.classList.remove('syncing')
    btn.title = isError ? `重建失败：${msg}` : '重建知识库索引'
  }
  const dropdown = document.getElementById('gs-dropdown')
  if (dropdown) {
    const color = isError ? '#cf222e' : '#1a7f37'
    _show(dropdown, `<div class="gs-status" style="color:${color}">${
      isError ? '❌ 重建失败：' : '✅ 重建完成：'
    }${_esc(msg || (isError ? '未知错误' : ''))}</div>`)
  }
}

// ── Search history ────────────────────────────────────────────────────────────

const _HIST_MAX = 10

function _getHistory(mode) {
  try {
    return JSON.parse(localStorage.getItem(`gs-history-${mode}`)) || []
  } catch (_) {
    return []
  }
}

function _addHistory(mode, q) {
  if (!q) return
  const list = _getHistory(mode).filter(x => x !== q)
  list.unshift(q)
  localStorage.setItem(`gs-history-${mode}`, JSON.stringify(list.slice(0, _HIST_MAX)))
}

function _removeHistory(mode, q) {
  const list = _getHistory(mode).filter(x => x !== q)
  localStorage.setItem(`gs-history-${mode}`, JSON.stringify(list))
}

function _renderHistory(mode) {
  const list = _getHistory(mode)
  if (!list.length) return ''
  const items = list.map(q => `
    <div class="gs-hist-item" data-mode="${mode}" data-q="${_esc(q)}">
      <span class="gs-hist-label">${_esc(q)}</span>
      <button class="gs-hist-remove" data-mode="${mode}" data-q="${_esc(q)}" title="删除">×</button>
    </div>
  `).join('')
  return `<div class="gs-hist-list">${items}</div>`
}

function _showHistory(mode) {
  const dropdown = document.getElementById('gs-dropdown')
  if (!dropdown) return
  const html = _renderHistory(mode)
  if (!html) return
  _show(dropdown, html)
  dropdown.querySelectorAll('.gs-hist-item').forEach(el => {
    el.addEventListener('click', e => {
      if (e.target.classList.contains('gs-hist-remove')) return
      const input = document.getElementById('gs-input')
      if (!input) return
      const q = el.dataset.q
      input.value = el.dataset.mode === 'wb' ? `#${q}` : q
      input.dispatchEvent(new Event('input', { bubbles: true }))
    })
  })
  dropdown.querySelectorAll('.gs-hist-remove').forEach(btn => {
    btn.addEventListener('click', e => {
      e.stopPropagation()
      _removeHistory(btn.dataset.mode, btn.dataset.q)
      _showHistory(mode)
    })
  })
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
      const { q: wbQ } = _detectMode(document.getElementById('gs-input')?.value || '')
      _addHistory('wb', wbQ)
      _close()
    })
  })

  // Attach click handlers for kb hits
  dropdown.querySelectorAll('.gs-hit-kb').forEach(el => {
    el.addEventListener('click', () => {
      const repo = el.dataset.repo
      const path = el.dataset.path
      const url = el.dataset.url
      const title = el.dataset.title
      if (repo && path) {
        document.dispatchEvent(new CustomEvent('cta:open-kb-doc', {
          detail: { repo, path, url, title }
        }))
      }
      const { q: kbQ } = _detectMode(document.getElementById('gs-input')?.value || '')
      _addHistory('kb', kbQ)
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
