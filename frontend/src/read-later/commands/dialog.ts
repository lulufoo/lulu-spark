import { readLaterOpenStore } from '../state/dialog-open.ts';

export function closeReadLaterDialog() {
  readLaterOpenStore.set(false);
  document.getElementById('read-later-dialog')?.classList.remove('open');
}

export function openReadLaterDialog() {
  readLaterOpenStore.set(true);
  document.getElementById('read-later-dialog')?.classList.add('open');
}
