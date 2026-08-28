import { convertStore, type ConvertTab } from '../state/convert.ts';

export { convertStore };

function setActive(el: HTMLElement | null, on: boolean) {
  if (!el) return;
  if (on) el.classList.add('active');
  else el.classList.remove('active');
}

function applyTabDom(name: ConvertTab) {
  setActive(document.getElementById('convert-tab-base64'), name === 'base64');
  setActive(document.getElementById('convert-tab-qr'), name === 'qr');
  setActive(document.getElementById('convert-panel-base64'), name === 'base64');
  setActive(document.getElementById('convert-panel-qr'), name === 'qr');
}

export function setConvertTab(tab: string) {
  const name: ConvertTab = tab === 'qr' ? 'qr' : 'base64';
  convertStore.set((s) => ({ ...s, tab: name }));
  applyTabDom(name);
}

export function openConvertDialog(tab = 'base64') {
  const name: ConvertTab = tab === 'qr' ? 'qr' : 'base64';
  convertStore.set({ open: true, tab: name });
  document.getElementById('convert-dialog')?.classList.add('open');
  applyTabDom(name);
  if (name === 'qr') document.getElementById('qr-input')?.focus();
  else document.getElementById('base64-input')?.focus();
}

export function closeConvertDialog() {
  convertStore.set((s) => ({ ...s, open: false }));
  document.getElementById('convert-dialog')?.classList.remove('open');
}
