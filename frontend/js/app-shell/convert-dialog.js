function setActive(el, on) {
  if (on) el.classList.add('active');
  else el.classList.remove('active');
}

export function setConvertTab(tab) {
  const name = tab === 'qr' ? 'qr' : 'base64';
  setActive(document.getElementById('convert-tab-base64'), name === 'base64');
  setActive(document.getElementById('convert-tab-qr'), name === 'qr');
  setActive(document.getElementById('convert-panel-base64'), name === 'base64');
  setActive(document.getElementById('convert-panel-qr'), name === 'qr');
}

export function openConvertDialog(tab = 'base64') {
  setConvertTab(tab);
  document.getElementById('convert-dialog').classList.add('open');
  if (tab === 'qr') document.getElementById('qr-input').focus();
  else document.getElementById('base64-input').focus();
}

export function closeConvertDialog() {
  document.getElementById('convert-dialog').classList.remove('open');
}

document.getElementById('btn-convert-close').addEventListener('click', closeConvertDialog);
document.getElementById('convert-tab-base64').addEventListener('click', () => {
  setConvertTab('base64');
  document.getElementById('base64-input').focus();
});
document.getElementById('convert-tab-qr').addEventListener('click', () => {
  setConvertTab('qr');
  document.getElementById('qr-input').focus();
});
