/**
 * @vitest-environment jsdom
 */
import { describe, it, expect, beforeAll } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));

function markVisible(el) {
  el.getBoundingClientRect = () => ({
    width: 120,
    height: 40,
    top: 0,
    bottom: 40,
    left: 0,
    right: 120,
  });
}

beforeAll(() => {
  const src = readFileSync(
    resolve(__dirname, '../../extensions/chrome-spark-extension/x-zh-en/compose-dom.js'),
    'utf8',
  );
  window.eval(src);
});

describe('readEditor ignores compose hint', () => {
  it('returns empty when only Draft placeholder 有什么新鲜事？ is visible', () => {
    const wrap = document.createElement('div');
    wrap.setAttribute('data-testid', 'tweetTextarea_0');
    const hint = document.createElement('div');
    hint.className = 'public-DraftEditorPlaceholder-inner';
    hint.textContent = '有什么新鲜事？';
    const box = document.createElement('div');
    box.setAttribute('contenteditable', 'true');
    box.setAttribute('role', 'textbox');
    box.innerHTML = '<br>';
    wrap.appendChild(hint);
    wrap.appendChild(box);
    markVisible(wrap);
    markVisible(box);
    document.body.appendChild(wrap);

    expect(window.Xzh.readEditor(wrap)).toBe('');
  });

  it('returns typed text and not the sibling hint', () => {
    const wrap = document.createElement('div');
    wrap.setAttribute('data-testid', 'tweetTextarea_0');
    const hint = document.createElement('div');
    hint.className = 'public-DraftEditorPlaceholder-inner';
    hint.textContent = '有什么新鲜事？';
    const box = document.createElement('div');
    box.setAttribute('contenteditable', 'true');
    box.textContent = '你好世界';
    wrap.appendChild(hint);
    wrap.appendChild(box);
    markVisible(wrap);
    markVisible(box);
    document.body.appendChild(wrap);

    expect(window.Xzh.readEditor(wrap)).toBe('你好世界');
  });
});
