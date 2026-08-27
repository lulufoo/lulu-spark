// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { initKbComments, cleanupKbComments } from '../../frontend/js/corpus/corpus-comments.js';
import {
  cleanupDocHighlightOverlay,
  initDocHighlightOverlay,
} from '../../frontend/js/doc-editor/highlights.js';

function bindKbHighlightForTest(reader) {
  initDocHighlightOverlay({
    getBody: () => reader.querySelector('.kb-reader-body'),
    getEditArea: () => reader.querySelector('.kb-reader-edit-area'),
    getIdentityKey: () => '',
    excludeBarId: 'kb-md-comments-bar',
    buttonId: 'kb-highlight-add-btn',
  });
}

describe('kb comments/highlights scope', () => {
  beforeEach(() => {
    document.body.innerHTML = `
      <div id="kb-comment-dialog">
        <button id="kb-btn-comment-cancel"></button>
        <button id="kb-btn-comment-save"></button>
        <div id="kb-comment-dialog-content"></div>
        <div id="kb-comment-editor-box"></div>
        <div id="kb-comment-preview-pane"></div>
        <button class="comment-tab-btn" data-tab="edit"></button>
        <button class="comment-tab-btn" data-tab="preview"></button>
      </div>
    `;
  });

  it('init/cleanup cycle does not stack add-comment listeners', () => {
    const root = document.createElement('div');
    root.innerHTML = `
      <div class="kb-reader">
        <div class="kb-reader-body"></div>
        <div class="kb-comment-float-nav"></div>
        <button class="kb-btn-add-comment"></button>
      </div>
    `;
    document.body.appendChild(root);
    const reader = root.querySelector('.kb-reader');
    const addBtn = reader.querySelector('.kb-btn-add-comment');
    const addSpy = vi.spyOn(addBtn, 'addEventListener');
    const removeSpy = vi.spyOn(addBtn, 'removeEventListener');

    initKbComments(reader);
    initKbComments(reader);
    cleanupKbComments();
    initKbComments(reader);

    expect(addSpy.mock.calls.filter(([type]) => type === 'click')).toHaveLength(3);
    expect(removeSpy.mock.calls.filter(([type]) => type === 'click')).toHaveLength(2);
    addSpy.mockRestore();
    removeSpy.mockRestore();
    cleanupKbComments();
  });

  it('highlight init/cleanup can repeat without throwing', () => {
    const root = document.createElement('div');
    root.innerHTML = `
      <div class="kb-reader">
        <div class="kb-reader-body"><p>text</p></div>
      </div>
    `;
    document.body.appendChild(root);
    const reader = root.querySelector('.kb-reader');

    for (let i = 0; i < 3; i += 1) {
      bindKbHighlightForTest(reader);
      cleanupDocHighlightOverlay();
    }
  });

  it('does not duplicate #kb-highlight-add-btn and hides it on click', async () => {
    document.body.insertAdjacentHTML(
      'beforeend',
      '<button id="kb-highlight-add-btn" style="display:none">高亮</button>',
    );
    const root = document.createElement('div');
    root.innerHTML = `
      <div class="kb-reader">
        <div class="kb-reader-body"><p>hello world</p></div>
      </div>
    `;
    document.body.appendChild(root);
    const reader = root.querySelector('.kb-reader');

    bindKbHighlightForTest(reader);
    bindKbHighlightForTest(reader);

    expect(document.querySelectorAll('#kb-highlight-add-btn')).toHaveLength(1);
    const btn = document.getElementById('kb-highlight-add-btn');
    btn.style.display = 'block';

    btn.click();
    expect(btn.style.display).toBe('none');

    cleanupDocHighlightOverlay();
  });
});
