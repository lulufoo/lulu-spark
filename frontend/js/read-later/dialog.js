import { mountReadLaterList } from './list.js';

let unmountList = null;

export function closeReadLaterDialog() {
  const dialog = document.getElementById('read-later-dialog');
  dialog?.classList.remove('open');
  unmountList?.();
  unmountList = null;
}

export function openReadLaterDialog() {
  const dialog = document.getElementById('read-later-dialog');
  const body = document.getElementById('read-later-dialog-body');
  if (!dialog || !body) return;

  unmountList?.();
  unmountList = mountReadLaterList(body, { showTabs: true, initialFilter: 'unread' }).unmount;
  dialog.classList.add('open');
}

function wireReadLaterDialog() {
  const dialog = document.getElementById('read-later-dialog');
  if (!dialog || dialog.dataset.wired === '1') return;
  dialog.dataset.wired = '1';

  document.getElementById('btn-read-later-close')?.addEventListener('click', closeReadLaterDialog);
  dialog.addEventListener('click', (event) => {
    if (event.target === dialog) closeReadLaterDialog();
  });
}

wireReadLaterDialog();
