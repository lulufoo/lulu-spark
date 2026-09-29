export function openQrDialog() {
  (document.getElementById('qr-input') as HTMLInputElement).value = '';
  document.getElementById('qr-preview')!.innerHTML = '';
  document.getElementById('convert-dialog')?.classList.add('open');
  document.getElementById('qr-input')?.focus();
}
