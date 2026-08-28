import { openConvertDialog } from './convert-dialog.tsx';
import { renderToHtml } from '../island.ts';

declare const QRCode: {
  toCanvas: (
    canvas: HTMLCanvasElement,
    text: string,
    opts: { width: number; margin: number },
    cb: (err?: Error | null) => void,
  ) => void;
  toDataURL: (
    text: string,
    opts: { width: number; margin: number },
    cb: (err: Error | null | undefined, url?: string) => void,
  ) => void;
};

const QR_OPTS = { width: 256, margin: 2 };
const OVERFLOW_MSG = '⚠️ Text too long to generate QR code (capacity ~2 KB UTF-8)';

function isOverflowError(err: unknown) {
  const msg = String((err as Error | undefined)?.message || err || '');
  return /overflow|too big|too large/i.test(msg);
}

function showQrError(preview: HTMLElement, message = OVERFLOW_MSG) {
  preview.innerHTML = renderToHtml(<p className="qr-error">{message}</p>);
}

export function openQrDialog() {
  (document.getElementById('qr-input') as HTMLInputElement).value = '';
  document.getElementById('qr-preview')!.innerHTML = '';
  openConvertDialog('qr');
}

export function renderQr(text: string) {
  const preview = document.getElementById('qr-preview')!;
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
        if (err2 || !url) {
          showQrError(preview);
          return;
        }
        preview.innerHTML = renderToHtml(<img src={url} alt="QR" />);
      });
      return;
    }
    preview.appendChild(canvas);
  });
}

document.getElementById('qr-input')?.addEventListener('input', () => {
  renderQr((document.getElementById('qr-input') as HTMLInputElement).value);
});
