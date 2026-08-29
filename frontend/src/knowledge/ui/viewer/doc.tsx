import { renderToHtml } from '../../../island.ts';

export function paintKbDocLoading(body: HTMLElement) {
  body.innerHTML = renderToHtml(
    <div style={{ color: '#8c959f', padding: 20, fontSize: 13 }}>Loading…</div>,
  );
}

export function paintKbDocPlain(body: HTMLElement, text: string) {
  body.innerHTML = renderToHtml(
    <pre style={{ whiteSpace: 'pre-wrap', fontSize: 13 }}>{text}</pre>,
  );
}

export function paintKbDocError(body: HTMLElement, message: string) {
  body.innerHTML = renderToHtml(
    <div style={{ color: '#7d4e00', padding: 20 }}>Could not load file: {message}</div>,
  );
}
