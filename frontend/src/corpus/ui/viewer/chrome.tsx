import * as api from '../../../host/api.ts';

let _kbPendingMsg = '';

function kbPendingBtn() {
  return (
    document.getElementById('kb-btn-pending')
    || document.querySelector('.kb-btn-pending')
  ) as HTMLButtonElement | null;
}

function kbReindexBtn() {
  return (
    document.getElementById('kb-btn-reindex')
    || document.querySelector('.kb-btn-reindex')
  ) as HTMLButtonElement | null;
}

export function kbPendingMsg() {
  return _kbPendingMsg;
}

export function onKbDirty(e: Event) {
  const detail = (e as CustomEvent<{ msg?: string }>).detail;
  kbShowPendingBadge(detail?.msg || 'chore: update via viewer');
}

export function kbShowPendingBadge(msg: string) {
  _kbPendingMsg = msg;
  const btn = kbPendingBtn();
  if (btn) btn.style.display = '';
}

export function kbHidePendingBadge() {
  _kbPendingMsg = '';
  const btn = kbPendingBtn();
  if (btn) btn.style.display = 'none';
}

export function showKbReindexBtn(repo: string) {
  const btn = kbReindexBtn();
  if (!btn) return;
  btn.textContent = '↺ Rebuild index';
  btn.title = 'Rebuild search index for this library';
  btn.disabled = false;
  btn.style.display = '';
  btn.onclick = async () => {
    btn.disabled = true;
    btn.textContent = 'Rebuilding…';
    try {
      const res = (await api.reindexKbRepo(repo)) as { error?: string };
      if (res.error) throw new Error(res.error);
      const poll = setInterval(async () => {
        try {
          const status = (await api.getReindexStatus()) as { status?: string; log?: string };
          if (status.status === 'done') {
            clearInterval(poll);
            btn.textContent = '✓ Rebuilt';
            btn.disabled = false;
            setTimeout(() => {
              btn.style.display = 'none';
            }, 2000);
          } else if (status.status === 'error') {
            clearInterval(poll);
            btn.textContent = 'Rebuild failed';
            btn.disabled = false;
            btn.title = status.log || 'Unknown error';
          }
        } catch {
          // keep polling
        }
      }, 2000);
    } catch (err) {
      btn.textContent = 'Rebuild failed';
      btn.disabled = false;
      btn.title = err instanceof Error ? err.message : String(err);
    }
  };
}
