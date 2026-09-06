let _kbPendingMsg = '';

function kbPendingBtn() {
  return (
    document.getElementById('kb-btn-pending')
    || document.querySelector('.kb-btn-pending')
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
