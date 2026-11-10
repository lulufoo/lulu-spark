// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { SketchPreview } from '../../frontend/src/board/ui/sketch-preview.tsx';
import {
  appearanceMessage,
  isBoardPage,
  readSandboxMessage,
  renderMessage,
} from '../../frontend/src/board/commands/sketch-preview.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const PAGE = 'https://luluboard.app/view?utm=1#b:b_0123abcd';
const HTML = '<p>Hello</p>';

function render(props) {
  const host = document.createElement('div');
  document.body.appendChild(host);
  const root = createRoot(host);
  act(() => root.render(createElement(SketchPreview, props)));
  return { host, root };
}

function postFromFrame(frame, data) {
  frame.contentWindow.postMessage = () => {};
  window.dispatchEvent(new MessageEvent('message', { data, source: frame.contentWindow }));
}

beforeEach(() => {
  document.body.innerHTML = '';
  delete window.__TAURI__;
});

describe('sketch preview', () => {
  it('embeds the sandbox runner without same-origin and without a token', () => {
    const { host, root } = render({ html: HTML, pageUrl: PAGE, appearance: 'light' });
    const frame = host.querySelector('iframe');
    expect(frame.getAttribute('src')).toBe('https://sandbox.luluboard.app/');
    expect(frame.getAttribute('sandbox')).toBe('allow-scripts');
    expect(frame.getAttribute('sandbox')).not.toContain('allow-same-origin');
    expect(frame.style.colorScheme).toBe('light');
    const css = readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');
    const previewRule = css.match(/\.board-sketch-preview\s*\{[^}]*\}/)[0];
    expect(previewRule).toMatch(/border:\s*1px solid var\(--border-default\)/);
    expect(previewRule).toMatch(/border-radius:\s*9px/);
    expect(previewRule).toMatch(/padding:\s*0/);
    const titleRule = css.match(/\.board-sketch-title\s*\{[^}]*\}/)[0];
    expect(titleRule).toMatch(/border-bottom:\s*1px solid var\(--border-default\)/);
    expect(titleRule).toMatch(/background:\s*var\(--bg-surface\)/);
    expect(host.querySelector('header.board-sketch-title .board-sketch-name')).not.toBeNull();
    expect(css).toMatch(/p:has\(>\s*a\[hidden\] \+ \.board-sketch-slot\)/);
    expect(css).toMatch(/\.board-sketch-frame\s*\{[^}]*color-scheme:\s*light/);
    expect(css).not.toMatch(/\[data-theme="dark"\] \.board-sketch-frame\s*\{[^}]*color-scheme:\s*dark/);
    const csp = JSON.parse(readFileSync(join(repoRoot, 'src-tauri/tauri.conf.json'), 'utf8')).app.security.csp;
    expect(csp).toContain('frame-src https://sandbox.luluboard.app');
    expect(csp).not.toContain('frame-src *');
    expect(host.textContent).not.toContain('lbp_');
    act(() => root.unmount());
  });

  it('sends the fragment after ready and opens only the Board page', async () => {
    const opened = [];
    window.__TAURI__ = { opener: { openUrl: async (url) => { opened.push(url); } } };
    const { host, root } = render({ html: HTML, pageUrl: PAGE, appearance: 'dark' });
    const frame = host.querySelector('iframe');
    expect(frame.style.colorScheme).toBe('light');
    const posted = [];
    frame.contentWindow.postMessage = (data) => posted.push(data);
    act(() => {
      window.dispatchEvent(new MessageEvent('message', {
        data: { type: 'lulu-sandbox:ready', pat: 'lbp_secret' },
        source: frame.contentWindow,
      }));
    });
    expect(posted).toEqual([renderMessage(HTML, 'dark')]);
    expect(JSON.stringify(posted)).not.toContain('lbp_');
    act(() => {
      window.dispatchEvent(new MessageEvent('message', {
        data: { type: 'lulu-sandbox:ready' },
        source: window,
      }));
    });
    expect(posted).toHaveLength(1);
    expect(host.querySelector('.board-sketch-open').textContent).toBe('Open in Board ↗');
    expect(host.querySelector('.board-sketch-hit')).toBeNull();
    host.querySelector('.board-sketch-frame-box').click();
    await act(async () => { await Promise.resolve(); });
    expect(opened).toEqual([]);
    host.querySelector('.board-sketch-open').click();
    await act(async () => { await Promise.resolve(); });
    expect(opened).toEqual([PAGE]);
    act(() => root.render(createElement(SketchPreview, { html: HTML, pageUrl: 'https://evil.test', appearance: 'dark' })));
    expect(host.querySelector('button')).toBeNull();
    act(() => root.unmount());
  });

  it('shows a load failure and follows a later theme change', () => {
    const { host, root } = render({ html: HTML, pageUrl: PAGE, appearance: 'light' });
    const frame = host.querySelector('iframe');
    const posted = [];
    frame.contentWindow.postMessage = (data) => posted.push(data);
    act(() => {
      window.dispatchEvent(new MessageEvent('message', {
        data: { type: 'lulu-sandbox:error', message: '<script>' },
        source: frame.contentWindow,
      }));
    });
    expect(host.querySelector('.board-sketch-error').textContent).toBe('The sketch preview failed to load.');
    expect(host.textContent).not.toContain('<script>');
    act(() => {
      window.dispatchEvent(new MessageEvent('message', {
        data: { type: 'lulu-sandbox:ready' },
        source: frame.contentWindow,
      }));
    });
    posted.length = 0;
    act(() => root.render(createElement(SketchPreview, { html: HTML, pageUrl: PAGE, appearance: 'dark' })));
    expect(posted).toEqual([appearanceMessage('dark')]);
    expect(host.querySelector('iframe').style.colorScheme).toBe('light');
    expect(isBoardPage(PAGE)).toBe(true);
    expect(isBoardPage('https://evil.test/#b:b_0123abcd')).toBe(false);
    expect(readSandboxMessage({ type: 'lulu-sandbox:size', width: 10, height: 20 })?.width).toBe(10);
    expect(readSandboxMessage({ type: 'lulu-sandbox:size', width: 0, height: 20 })).toBeNull();
    const box = host.querySelector('.board-sketch-frame-box');
    Object.defineProperty(box, 'clientWidth', { configurable: true, get: () => 120 });
    act(() => {
      window.dispatchEvent(new MessageEvent('message', {
        data: { type: 'lulu-sandbox:size', width: 480, height: 200 },
        source: frame.contentWindow,
      }));
    });
    const scaled = host.querySelector('iframe');
    expect(scaled.style.transform).toBe('scale(0.25)');
    expect(box.style.height).toBe('50px');
    expect(host.querySelector('.board-sketch-error')).toBeNull();
    act(() => root.unmount());
  });

  it('reports a failure when the runner stays blank after it is ready', async () => {
    vi.useFakeTimers();
    try {
      const host = document.createElement('div');
      const link = document.createElement('a');
      link.href = PAGE;
      link.textContent = 'Sketch';
      const slot = document.createElement('div');
      host.append(link, slot);
      document.body.appendChild(host);
      const root = createRoot(slot);
      act(() => root.render(createElement(SketchPreview, { html: HTML, pageUrl: PAGE, appearance: 'light' })));
      const frame = host.querySelector('iframe');
      frame.contentWindow.postMessage = () => {};
      act(() => {
        window.dispatchEvent(new MessageEvent('message', {
          data: { type: 'lulu-sandbox:ready' },
          source: frame.contentWindow,
        }));
      });
      expect(host.querySelector('.board-sketch-error')).toBeNull();
      await act(async () => { await vi.advanceTimersByTimeAsync(8000); });
      expect(host.querySelector('.board-sketch-error').textContent).toBe('The sketch preview failed to load.');
      expect(host.querySelector('a').textContent).toBe('Sketch');
      act(() => root.unmount());
    } finally {
      vi.useRealTimers();
    }
  });
});
