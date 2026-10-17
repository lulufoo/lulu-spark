// @vitest-environment jsdom
import * as require_fs from 'node:fs';
import { readFileSync } from 'node:fs';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { TIP_DELAY_MS, TIP_GAP_PX, initTooltip, placeTip } from '../../frontend/src/shared/tooltip/tooltip.ts';
import { repoRoot } from '../helpers/read-frontend-js.js';

function rect(partial) {
  return { x: 0, y: 0, top: 0, bottom: 0, left: 0, right: 0, width: 0, height: 0, toJSON() {}, ...partial };
}

describe('shared tooltip (data-tip)', () => {
  let stop;
  let tip;

  function over(el, relatedTarget = null) {
    el.dispatchEvent(new MouseEvent('mouseover', { bubbles: true, relatedTarget }));
  }
  function out(el, relatedTarget = document.body) {
    el.dispatchEvent(new MouseEvent('mouseout', { bubbles: true, relatedTarget }));
  }

  beforeEach(() => {
    vi.useFakeTimers();
    document.body.innerHTML = `
      <button id="a" data-tip="12 projects collapsed"><span id="inner">…</span></button>
      <span id="cut" data-tip="A very long workspace name" data-tip-truncated>A very long workspace name</span>
      <button id="plain">no tip</button>`;
    stop = initTooltip();
    tip = document.querySelector('[data-role="tip"]');
  });

  afterEach(() => {
    stop();
    vi.useRealTimers();
    document.body.innerHTML = '';
  });

  it('is one shared look: the .ui-tip box, hidden until a hover settles', () => {
    expect(tip.className).toBe('ui-tip');
    expect(tip.style.display).toBe('none');
    over(document.getElementById('a'));
    expect(tip.style.display).toBe('none');
    vi.advanceTimersByTime(TIP_DELAY_MS - 1);
    expect(tip.style.display).toBe('none');
    vi.advanceTimersByTime(1);
    expect(tip.style.display).toBe('block');
    expect(tip.textContent).toBe('12 projects collapsed');
  });

  it('keeps showing while the pointer moves between children, and hides on leave', () => {
    const a = document.getElementById('a');
    const inner = document.getElementById('inner');
    over(a);
    vi.advanceTimersByTime(TIP_DELAY_MS);
    over(inner);
    out(a, inner);
    expect(tip.style.display).toBe('block');
    out(inner, document.body);
    expect(tip.style.display).toBe('none');
  });

  it('does not show for an element without a tip, and cancels when the pointer leaves early', () => {
    over(document.getElementById('plain'));
    vi.advanceTimersByTime(500);
    expect(tip.style.display).toBe('none');
    const a = document.getElementById('a');
    over(a);
    vi.advanceTimersByTime(TIP_DELAY_MS - 20);
    out(a);
    vi.advanceTimersByTime(500);
    expect(tip.style.display).toBe('none');
  });

  it('hides on press, key and scroll', () => {
    const a = document.getElementById('a');
    for (const fire of [
      () => a.dispatchEvent(new MouseEvent('mousedown', { bubbles: true })),
      () => document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' })),
      () => document.dispatchEvent(new Event('scroll')),
    ]) {
      over(a);
      vi.advanceTimersByTime(TIP_DELAY_MS);
      expect(tip.style.display).toBe('block');
      fire();
      expect(tip.style.display).toBe('none');
      out(a);
    }
  });

  it('data-tip-truncated only shows when the text is cut off', () => {
    const cut = document.getElementById('cut');
    Object.defineProperty(cut, 'scrollWidth', { configurable: true, value: 100 });
    Object.defineProperty(cut, 'clientWidth', { configurable: true, value: 100 });
    over(cut);
    vi.advanceTimersByTime(TIP_DELAY_MS);
    expect(tip.style.display).toBe('none');
    out(cut);
    Object.defineProperty(cut, 'scrollWidth', { configurable: true, value: 180 });
    over(cut);
    vi.advanceTimersByTime(TIP_DELAY_MS);
    expect(tip.style.display).toBe('block');
  });
});

describe('placeTip', () => {
  const boxOf = (width, height) => {
    const el = document.createElement('div');
    el.getBoundingClientRect = () => rect({ width, height });
    return el;
  };

  it('sits below the element, left aligned', () => {
    Object.assign(window, { innerWidth: 1000, innerHeight: 800 });
    const el = boxOf(200, 30);
    placeTip(el, rect({ left: 100, right: 160, top: 100, bottom: 124 }));
    expect(el.style.left).toBe('100px');
    expect(el.style.top).toBe(`${124 + TIP_GAP_PX}px`);
  });

  it('flips above when there is no room below, and stays inside the window', () => {
    Object.assign(window, { innerWidth: 500, innerHeight: 400 });
    const el = boxOf(200, 30);
    placeTip(el, rect({ left: 480, right: 500, top: 380, bottom: 396 }));
    expect(el.style.top).toBe(`${380 - 30 - TIP_GAP_PX}px`);
    expect(el.style.left).toBe('296px');
  });
});

describe('one tooltip, no per-place copies (gate)', () => {
  function walk(dir, out = []) {
    for (const name of require_fs.readdirSync(dir)) {
      const p = `${dir}/${name}`;
      if (require_fs.statSync(p).isDirectory()) walk(p, out);
      else if (/\.(tsx|ts)$/.test(name)) out.push(p);
    }
    return out;
  }

  it('no native title= hover hints in the frontend source', () => {
    const offenders = [];
    for (const file of walk(`${repoRoot}/frontend/src`)) {
      const lines = readFileSync(file, 'utf8').split('\n');
      lines.forEach((line, i) => {
        if (/<DocCard\b/.test(line)) return; // DocCard has a `title` prop, not an attribute
        if (/\btitle=/.test(line) || /\.title\s*=\s*['"`]/.test(line) || /setAttribute\(\s*['"]title['"]/.test(line)) {
          offenders.push(`${file.replace(repoRoot + '/', '')}:${i + 1}`);
        }
      });
    }
    expect(offenders, 'use data-tip (shared/tooltip), not the native title').toEqual([]);
  });

  it('app.css defines tooltip styling only as .ui-tip, .ui-tip-preview and the comment preview anchor', () => {
    const css = readFileSync(`${repoRoot}/frontend/app.css`, 'utf8');
    const selectors = [...css.matchAll(/^[ \t]*([^{}\n@/][^{}\n]*)\{/gm)].map((m) => m[1].trim());
    const tipLike = selectors.filter((s) => /(^|[\s.#,])[\w-]*(tooltip|-tip|_tip)\b/i.test(s) || /\btip\b/i.test(s));
    const allowed = new Set(['.ui-tip', '.ui-tip.ui-tip-preview', '#comment-preview-tip']);
    expect(tipLike.filter((s) => !allowed.has(s))).toEqual([]);
  });

  it('the old per-place tooltip code is gone', () => {
    expect(() => readFileSync(`${repoRoot}/frontend/src/app-shell/ui/tooltip.ts`, 'utf8')).toThrow();
    const css = readFileSync(`${repoRoot}/frontend/app.css`, 'utf8');
    expect(css).not.toMatch(/#_tip|file-popup-path-tip|\.digest-tooltip/);
  });
});
