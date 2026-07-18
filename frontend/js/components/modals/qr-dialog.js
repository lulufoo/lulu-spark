const QR_OPTS = { width: 256, margin: 2 };
const OVERFLOW_MSG = '⚠️ Text too long to generate QR code (capacity ~2 KB UTF-8)';

function isOverflowError(err) {
  const msg = String(err?.message || err || '');
  return /overflow|too big|too large/i.test(msg);
}

function showQrError(preview, message = OVERFLOW_MSG) {
  preview.innerHTML = `<p class="qr-error">${message}</p>`;
}

export function openQrDialog() {
  document.getElementById('qr-input').value = '';
  document.getElementById('qr-preview').innerHTML = '';
  document.getElementById('qr-dialog').classList.add('open');
  document.getElementById('qr-input').focus();
}

export function renderQr(text) {
  const preview = document.getElementById('qr-preview');
  preview.innerHTML = '';
  const trimmed = (text ?? '').trim();
  if (!trimmed) return;

  const canvas = document.createElement('canvas');
  QRCode.toCanvas(canvas, trimmed, QR_OPTS, (err) => {
    if (err) {
      if (isOverflowError(err)) {
        showQrError(preview);
        return;
      }
      QRCode.toDataURL(trimmed, QR_OPTS, (err2, url) => {
        if (err2) {
          showQrError(preview);
          return;
        }
        preview.innerHTML = `<img src="${url}" alt="QR">`;
      });
      return;
    }
    preview.appendChild(canvas);
  });
}

document.getElementById('btn-qr-close').addEventListener('click', () => {
  document.getElementById('qr-dialog').classList.remove('open');
});

document.getElementById('qr-input').addEventListener('input', () => {
  renderQr(document.getElementById('qr-input').value);
});
