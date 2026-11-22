// @vitest-environment jsdom
import { createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { describe, it, expect, beforeAll, beforeEach, vi } from 'vitest';
import { readFrontendJs } from '../helpers/read-frontend-js.js';
import { state } from '../../frontend/src/host/state.ts';

vi.mock('../../frontend/src/host/api.ts', () => ({
  updateComments: vi.fn(),
}));

import * as api from '../../frontend/src/host/api.ts';
import {
  CommentDeleteDialog,
  confirmDeleteComment,
  confirmRemoveHighlight,
  removeNoteComment,
} from '../../frontend/src/shared/comment-delete.tsx';

function mountCommentDeleteDialog() {
  document.body.innerHTML = '<div id="comment-delete-host"></div>';
  const root = createRoot(document.getElementById('comment-delete-host'));
  flushSync(() => root.render(createElement(CommentDeleteDialog)));
}

beforeAll(() => {
  mountCommentDeleteDialog();
});

beforeEach(() => {
  vi.clearAllMocks();
  state.viewer = {
    kbId: 'abcdefabcdefabcdefabcdefabcdefab',
    annotation: {
      comments: [{ id: 'a17b9efe22cf', text: '是的', ts: '202605210050' }],
    },
  };
});

describe('confirmDeleteComment', () => {
  it('取消时返回 false 且不调用 API', async () => {
    const pending = confirmDeleteComment();
    document.getElementById('btn-comment-delete-cancel').click();
    await expect(pending).resolves.toBe(false);
    expect(api.updateComments).not.toHaveBeenCalled();
    expect(document.getElementById('comment-delete-dialog').classList.contains('open')).toBe(false);
  });

  it('确认时返回 true 并保留原有文案', async () => {
    const pending = confirmDeleteComment();
    await vi.waitFor(() => {
      expect(document.getElementById('comment-delete-dialog-title').textContent).toBe('Delete comment');
    });
    expect(document.getElementById('comment-delete-dialog-description').textContent)
      .toBe('Delete this comment? This cannot be undone.');
    expect(document.getElementById('btn-comment-delete-ok').textContent).toBe('Delete');
    document.getElementById('btn-comment-delete-ok').click();
    await expect(pending).resolves.toBe(true);
    expect(document.getElementById('comment-delete-dialog').classList.contains('open')).toBe(false);
  });

  it('点击遮罩时返回 false', async () => {
    const dialog = document.getElementById('comment-delete-dialog');
    const pending = confirmDeleteComment();
    const ev = new MouseEvent('click', { bubbles: true });
    Object.defineProperty(ev, 'target', { value: dialog });
    dialog.dispatchEvent(ev);
    await expect(pending).resolves.toBe(false);
  });
});

describe('confirmRemoveHighlight', () => {
  it('shows the highlight-specific warning and resolves true after Remove', async () => {
    const pending = confirmRemoveHighlight();
    await vi.waitFor(() => {
      expect(document.getElementById('comment-delete-dialog-title').textContent).toBe('Remove highlight');
    });
    expect(document.getElementById('comment-delete-dialog-description').textContent)
      .toContain('the highlight and its saved comments');
    const remove = document.getElementById('btn-comment-delete-ok');
    expect(remove.textContent).toBe('Remove');
    expect(document.activeElement).toBe(document.getElementById('btn-comment-delete-cancel'));
    remove.click();
    await expect(pending).resolves.toBe(true);
  });

  it('keeps keyboard focus inside and consumes Escape before dismissing', async () => {
    const trigger = document.createElement('button');
    const viewerKeyDown = vi.fn();
    document.body.appendChild(trigger);
    document.addEventListener('keydown', viewerKeyDown);
    try {
      const pending = confirmRemoveHighlight(trigger);
      const cancel = document.getElementById('btn-comment-delete-cancel');
      const remove = document.getElementById('btn-comment-delete-ok');
      expect(document.activeElement).toBe(cancel);
      cancel.dispatchEvent(new KeyboardEvent('keydown', {
        key: 'Tab', shiftKey: true, bubbles: true, cancelable: true,
      }));
      expect(document.activeElement).toBe(remove);
      remove.dispatchEvent(new KeyboardEvent('keydown', {
        key: 'Tab', bubbles: true, cancelable: true,
      }));
      expect(document.activeElement).toBe(cancel);

      viewerKeyDown.mockClear();
      cancel.dispatchEvent(new KeyboardEvent('keydown', {
        key: 'Escape', bubbles: true, cancelable: true,
      }));
      await expect(pending).resolves.toBe(false);
      expect(document.activeElement).toBe(trigger);
      expect(viewerKeyDown).not.toHaveBeenCalled();
    } finally {
      document.removeEventListener('keydown', viewerKeyDown);
      trigger.remove();
    }
  });

  it('cancels a superseded dialog and restores the original trigger after Cancel', async () => {
    const trigger = document.createElement('button');
    document.body.appendChild(trigger);
    try {
      trigger.focus();
      const first = confirmRemoveHighlight();
      const second = confirmDeleteComment();
      await expect(first).resolves.toBe(false);
      await vi.waitFor(() => {
        expect(document.getElementById('comment-delete-dialog-title').textContent).toBe('Delete comment');
      });
      const cancel = document.getElementById('btn-comment-delete-cancel');
      expect(document.activeElement).toBe(cancel);
      cancel.click();
      await expect(second).resolves.toBe(false);
      expect(document.activeElement).toBe(trigger);
    } finally {
      trigger.remove();
    }
  });
});

describe('removeNoteComment', () => {
  const entry = { id: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' };
  const layer = 'raw';
  const comment = { id: 'c1', text: 'note' };

  beforeEach(() => {
    state.viewer.annotation = {
      raw: { comments: [comment, { id: 'c2', text: 'keep' }] },
    };
  });

  it('成功时从 layer.comments 移除对应项', async () => {
    api.updateComments.mockResolvedValue({ ok: true });
    await removeNoteComment(comment, layer, entry);
    expect(api.updateComments).toHaveBeenCalledWith(
      'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
      'raw',
      { id: 'c1', text: '' },
      expect.any(String),
    );
    expect(state.viewer.annotation.raw.comments).toEqual([{ id: 'c2', text: 'keep' }]);
  });

  it('删除最后一条时移除 layer 键', async () => {
    state.viewer.annotation = { raw: { comments: [comment] } };
    api.updateComments.mockResolvedValue({ ok: true });
    await removeNoteComment(comment, layer, entry);
    expect(state.viewer.annotation.raw).toBeUndefined();
  });

  it('API 返回 ok:false 时抛出错误', async () => {
    api.updateComments.mockResolvedValue({ ok: false, error: 'Comment not found' });
    await expect(removeNoteComment(comment, layer, entry)).rejects.toThrow('Comment not found');
    expect(state.viewer.annotation.raw.comments).toHaveLength(2);
  });
});

describe('note delete does not use window.confirm', () => {
  it('notes comments source do not use window.confirm', () => {
    const notesSrc = readFrontendJs('frontend/src/notes/ui/comments.tsx');
    expect(notesSrc).not.toContain('window.confirm');
    expect(notesSrc).toContain('confirmDeleteComment');
  });
});
