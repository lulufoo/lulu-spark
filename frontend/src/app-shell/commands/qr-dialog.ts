import { convertStore } from '../state/convert.ts';

export function openQrDialog() {
  (document.getElementById('qr-input') as HTMLInputElement).value = '';
  document.getElementById('qr-preview')!.innerHTML = '';
  convertStore.set({ open: true, tab: 'qr' });
  document.getElementById('convert-dialog')?.classList.add('open');
  document.getElementById('qr-input')?.focus();
}
