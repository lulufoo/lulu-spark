import { openConvertDialog } from './convert-dialog.ts';

export function openQrDialog() {
  (document.getElementById('qr-input') as HTMLInputElement).value = '';
  document.getElementById('qr-preview')!.innerHTML = '';
  openConvertDialog('qr');
}
