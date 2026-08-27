// @vitest-environment jsdom
import { describe, it, expect, beforeAll, beforeEach, vi } from 'vitest';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { state } from '../../frontend/js/host/state.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const frontendJs = join(__dirname, '../../frontend/js');

vi.mock('../../frontend/js/host/api.js', () => ({
  updateKbComment: vi.fn(),
  updateComments: vi.fn(),
}));

import * as api from '../../frontend/js/host/api.js';
import {
  confirmDeleteComment,
  initCommentDeleteConfirm,
  removeKbComment,
  removeCorpusComment,
} from '../../frontend/js/shared/comment-delete.js';

function mountCommentDeleteDialog() {
  document.body.innerHTML = `
    <div id="comment-delete-dialog">
      <div id="comment-delete-dialog-box">
        <button type="button" id="btn-comment-delete-cancel">取消</button>
        <button type="button" id="btn-comment-delete-ok">删除</button>
      </div>
    </div>
  `;
  initCommentDeleteConfirm();
}

beforeAll(() => {
  mountCommentDeleteDialog();
});

beforeEach(() => {
  vi.clearAllMocks();
  state.viewer = {
    kbRepo: 'lulufoo/ai-collaboration-framework',
    kbPath: 'ai-dialogue-collab-framework/human-cognitive-prerequisites/2026-05-08-cognitive-abilities.md',
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
    expect(api.updateKbComment).not.toHaveBeenCalled();
    expect(api.updateComments).not.toHaveBeenCalled();
    expect(document.getElementById('comment-delete-dialog').classList.contains('open')).toBe(false);
  });

  it('确认时返回 true', async () => {
    const pending = confirmDeleteComment();
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

describe('removeKbComment', () => {
  it('成功时从 annotation.comments 移除对应项', async () => {
    api.updateKbComment.mockResolvedValue({ ok: true });
    await removeKbComment({ id: 'a17b9efe22cf' });
    expect(api.updateKbComment).toHaveBeenCalledWith(
      state.viewer.kbRepo,
      state.viewer.kbPath,
      { id: 'a17b9efe22cf', text: '' },
      expect.any(String),
    );
    expect(state.viewer.annotation.comments).toEqual([]);
  });

  it('API 返回 ok:false 时抛出错误', async () => {
    api.updateKbComment.mockResolvedValue({ ok: false, error: 'repo not found' });
    await expect(removeKbComment({ id: 'a17b9efe22cf' })).rejects.toThrow('repo not found');
    expect(state.viewer.annotation.comments).toHaveLength(1);
  });
});

describe('removeCorpusComment', () => {
  const entry = { common_path: 'ai/note.md' };
  const layer = 'raw';
  const comment = { id: 'c1', text: 'note' };

  beforeEach(() => {
    state.viewer.annotation = {
      raw: { comments: [comment, { id: 'c2', text: 'keep' }] },
    };
  });

  it('成功时从 layer.comments 移除对应项', async () => {
    api.updateComments.mockResolvedValue({ ok: true });
    await removeCorpusComment(comment, layer, entry);
    expect(api.updateComments).toHaveBeenCalledWith(
      'ai/note.md',
      'raw',
      { id: 'c1', text: '' },
      expect.any(String),
    );
    expect(state.viewer.annotation.raw.comments).toEqual([{ id: 'c2', text: 'keep' }]);
  });

  it('删除最后一条时移除 layer 键', async () => {
    state.viewer.annotation = { raw: { comments: [comment] } };
    api.updateComments.mockResolvedValue({ ok: true });
    await removeCorpusComment(comment, layer, entry);
    expect(state.viewer.annotation.raw).toBeUndefined();
  });

  it('API 返回 ok:false 时抛出错误', async () => {
    api.updateComments.mockResolvedValue({ ok: false, error: 'Comment not found' });
    await expect(removeCorpusComment(comment, layer, entry)).rejects.toThrow('Comment not found');
    expect(state.viewer.annotation.raw.comments).toHaveLength(2);
  });
});

describe('note delete does not use window.confirm', () => {
  it('corpus-comments and notes/comments source do not use window.confirm', () => {
    const kbSrc = readFileSync(join(frontendJs, 'corpus/corpus-comments.js'), 'utf8');
    const corpusSrc = readFileSync(join(frontendJs, 'notes/comments.js'), 'utf8');
    expect(kbSrc).not.toContain('window.confirm');
    expect(kbSrc).toContain('confirmDeleteComment');
    expect(corpusSrc).not.toContain('window.confirm');
    expect(corpusSrc).toContain('confirmDeleteComment');
  });
});
