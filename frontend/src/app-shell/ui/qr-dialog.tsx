import { openQrDialog } from '../commands/qr-dialog.ts';
import { renderToHtml } from '../../island.ts';

export { openQrDialog };

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

/** Node stub tests pass a plain object, not a real HTMLElement. */
export type QrPreviewHost = {
  innerHTML: string;
  appendChild: (child: HTMLCanvasElement) => unknown;
};

function isOverflowError(err: unknown) {
  const msg = String((err as Error | undefined)?.message || err || '');
  return /overflow|too big|too large/i.test(msg);
}

function showQrError(preview: QrPreviewHost, message = OVERFLOW_MSG) {
  preview.innerHTML = renderToHtml(<p className="qr-error">{message}</p>);
}

export function renderQr(text: string, preview: QrPreviewHost) {
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
