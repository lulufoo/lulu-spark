import * as api from '../../host/api.js'

let _kbPendingMsg = ''

export function kbPendingMsg() {
  return _kbPendingMsg
}

export function onKbDirty(e) {
  kbShowPendingBadge(e.detail?.msg || 'chore: update via viewer')
}

export function kbShowPendingBadge(msg) {
  _kbPendingMsg = msg
  const btn = document.getElementById('kb-btn-pending')
  if (btn) btn.style.display = ''
}

export function kbHidePendingBadge() {
  _kbPendingMsg = ''
  const btn = document.getElementById('kb-btn-pending')
  if (btn) btn.style.display = 'none'
}

// ── showKbReindexBtn ───────────────────────────────────────────────────────
export function showKbReindexBtn(repo) {
  let btn = document.getElementById('kb-btn-reindex');
  if (!btn) {
    btn = document.createElement('button');
    btn.id = 'kb-btn-reindex';
    btn.className = 'md-header-btn';
    const closeBtn = document.getElementById('kb-md-close');
    closeBtn.parentNode.insertBefore(btn, closeBtn);
  }
  btn.textContent = '↺ Rebuild index';
  btn.title = 'Rebuild search index for this library';
  btn.disabled = false;
  btn.style.display = '';
  btn.onclick = async () => {
    btn.disabled = true;
    btn.textContent = 'Rebuilding…';
    try {
      const res = await api.reindexKbRepo(repo);
      if (res.error) throw new Error(res.error);
      const poll = setInterval(async () => {
        try {
          const status = await api.getReindexStatus();
          if (status.status === 'done') {
            clearInterval(poll);
            btn.textContent = '✓ Rebuilt';
            btn.disabled = false;
            setTimeout(() => { btn.style.display = 'none'; }, 2000);
          } else if (status.status === 'error') {
            clearInterval(poll);
            btn.textContent = 'Rebuild failed';
            btn.disabled = false;
            btn.title = status.log || 'Unknown error';
          }
        } catch (_) {}
      }, 2000);
    } catch (e) {
      btn.textContent = 'Rebuild failed';
      btn.disabled = false;
      btn.title = e.message;
    }
  };
}
