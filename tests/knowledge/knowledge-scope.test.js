// @vitest-environment jsdom
import { createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { describe, it, expect, beforeEach, vi } from 'vitest';

vi.mock('../../frontend/src/host/api.ts', () => ({
  reorderKbComments: vi.fn(),
  updateKbComment: vi.fn(),
}));

import {
  initKbComments,
  cleanupKbComments,
  KbCommentDialog,
  openKbCommentDialog,
  closeKbCommentDialog,
} from '../../frontend/src/knowledge/ui/comments.tsx';
import {
  cleanupDocHighlightOverlay,
  initDocHighlightOverlay,
} from '../../frontend/src/doc-editor/highlights.ts';

function bindKbHighlightForTest(reader) {
  initDocHighlightOverlay({
    getBody: () => reader.querySelector('.kb-reader-body'),
    getEditArea: () => reader.querySelector('.kb-reader-edit-area'),
    getIdentityKey: () => '',
    excludeBarId: 'kb-md-comments-bar',
    buttonId: 'kb-highlight-add-btn',
  });
}

function mountKbCommentDialog() {
  const host = document.createElement('div');
  host.id = 'kb-comment-dialog-host';
  document.body.appendChild(host);
  const root = createRoot(host);
  flushSync(() => root.render(createElement(KbCommentDialog)));
}

describe('kb comments/highlights scope', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    mountKbCommentDialog();
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

  it('KbCommentDialog keep ids and Cancel closes via onClick', () => {
    const dialog = document.getElementById('kb-comment-dialog');
    expect(dialog).toBeTruthy();
    expect(document.getElementById('kb-comment-dialog-title').textContent).toMatch(/Add comment/);
    expect(document.getElementById('kb-comment-dialog-content').getAttribute('contenteditable')).toBe('true');
    expect(document.getElementById('kb-btn-comment-cancel')).toBeTruthy();
    expect(document.getElementById('kb-btn-comment-save')).toBeTruthy();

    openKbCommentDialog();
    expect(dialog.classList.contains('open')).toBe(true);

    document.getElementById('kb-btn-comment-cancel').click();
    expect(dialog.classList.contains('open')).toBe(false);

    openKbCommentDialog();
    closeKbCommentDialog();
    expect(dialog.classList.contains('open')).toBe(false);
  });

  it('preview tab hides editor and shows preview pane', async () => {
    openKbCommentDialog();
    document.getElementById('kb-comment-dialog-content').textContent = 'kb preview';
    document.querySelector('#kb-comment-dialog .comment-tab-btn[data-tab="preview"]').click();
    await Promise.resolve();

    expect(document.getElementById('kb-comment-editor-box').style.display).toBe('none');
    expect(document.getElementById('kb-comment-preview-pane').style.display).toBe('block');
    expect(document.getElementById('kb-comment-preview-pane').innerHTML).toMatch(/kb preview/);

    document.querySelector('#kb-comment-dialog .comment-tab-btn[data-tab="edit"]').click();
    expect(document.getElementById('kb-comment-editor-box').style.display).toBe('');
    expect(document.getElementById('kb-comment-preview-pane').style.display).toBe('none');
  });
});
