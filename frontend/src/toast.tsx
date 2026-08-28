import { flushSync } from 'react-dom';
import { createRoot, type Root } from 'react-dom/client';

const DISMISS_MS = 3000;

export type ToastType = 'success' | 'error';

let host: HTMLDivElement | null = null;
let root: Root | null = null;
let dismissTimer: ReturnType<typeof setTimeout> | null = null;

function ensureHost() {
  if (host?.isConnected && root) return;
  root?.unmount();
  host = document.createElement('div');
  host.id = 'react-toast-root';
  document.body.appendChild(host);
  root = createRoot(host);
}

function ToastView({ message, type }: { message: string; type: ToastType }) {
  const typeClass = type === 'error' ? 'wb-toast-error' : 'wb-toast-success';
  return (
    <div className={`wb-toast ${typeClass}`} role="status">
      {message}
    </div>
  );
}

/** Same contract as the former vanilla `showToast`. */
export function showToast(message: string, type: ToastType) {
  ensureHost();
  if (dismissTimer) clearTimeout(dismissTimer);
  flushSync(() => {
    root?.render(<ToastView message={message} type={type} />);
  });
  dismissTimer = setTimeout(() => {
    flushSync(() => {
      root?.render(null);
    });
    dismissTimer = null;
  }, DISMISS_MS);
}
