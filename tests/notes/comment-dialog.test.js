// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { state } from '../../frontend/src/host/state.ts';

vi.mock('../../frontend/src/host/api.ts', () => ({
  getDraft: vi.fn().mockResolvedValue({ content: '' }),
  saveDraft: vi.fn().mockResolvedValue({ ok: true }),
  updateComments: vi.fn(),
  reorderComments: vi.fn(),
}));

import {
  NoteCommentDialog,
  NotesDeleteZone,
  openCommentDialog,
  closeCommentDialog,
} from '../../frontend/src/notes/ui/comments.tsx';

function mountNoteCommentDialog() {
  document.body.innerHTML = '<div id="comment-dialog-host"></div>';
  const root = createRoot(document.getElementById('comment-dialog-host'));
  flushSync(() => root.render(createElement(NoteCommentDialog)));
}

describe('NoteCommentDialog', () => {
  beforeEach(() => {
    mountNoteCommentDialog();
    state.viewer.entry = { common_path: 'inbox/notes/demo.md' };
    state.viewer.layer = 'raw';
    state.viewer.annotation = {};
  });

  it('keeps English copy and comment-dialog ids', () => {
    expect(document.getElementById('comment-dialog')).toBeTruthy();
    expect(document.getElementById('comment-dialog-title').textContent).toBe('Add comment');
    expect(document.getElementById('comment-dialog-content').getAttribute('contenteditable')).toBe('true');
    expect(document.getElementById('comment-editor-box')).toBeTruthy();
    expect(document.getElementById('comment-preview-pane')).toBeTruthy();
    expect(document.getElementById('btn-comment-cancel').textContent).toBe('Cancel');
    expect(document.getElementById('btn-comment-save').textContent).toBe('Save');
    const tabs = [...document.querySelectorAll('#comment-dialog .comment-tab-btn')];
    expect(tabs.map((b) => b.dataset.tab)).toEqual(['edit', 'preview']);
    expect(tabs.map((b) => b.textContent)).toEqual(['Edit', 'Preview']);
  });

  it('openCommentDialog dual-writes classList open; Cancel and overlay close', async () => {
    const dialog = document.getElementById('comment-dialog');
    await openCommentDialog(null, null, null, 1);
    expect(dialog.classList.contains('open')).toBe(true);

    document.getElementById('btn-comment-cancel').click();
    expect(dialog.classList.contains('open')).toBe(false);

    await openCommentDialog(null, null, null, 1);
    expect(dialog.classList.contains('open')).toBe(true);
    dialog.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    const ev = new MouseEvent('click', { bubbles: true });
    Object.defineProperty(ev, 'target', { value: dialog });
    dialog.dispatchEvent(ev);
    expect(dialog.classList.contains('open')).toBe(false);

    await openCommentDialog(null, null, null, 1);
    closeCommentDialog();
    expect(dialog.classList.contains('open')).toBe(false);
  });

  it('preview tab hides editor and shows preview pane', async () => {
    const content = document.getElementById('comment-dialog-content');
    content.innerText = 'hello preview';
    await openCommentDialog(null, null, null, 1);
    content.innerText = 'hello preview';

    const previewBtn = document.querySelector('#comment-dialog .comment-tab-btn[data-tab="preview"]');
    previewBtn.click();
    await Promise.resolve();

    expect(document.getElementById('comment-editor-box').style.display).toBe('none');
    expect(document.getElementById('comment-preview-pane').style.display).toBe('block');
    expect(document.getElementById('comment-preview-pane').innerHTML).toMatch(/hello preview/);

    document.querySelector('#comment-dialog .comment-tab-btn[data-tab="edit"]').click();
    expect(document.getElementById('comment-editor-box').style.display).toBe('');
    expect(document.getElementById('comment-preview-pane').style.display).toBe('none');
  });

  it('delete button title carries no raw/digest layer wording', () => {
    const host = document.createElement('div');
    host.id = 'delete-zone-host';
    document.body.appendChild(host);
    const root = createRoot(host);
    flushSync(() => root.render(createElement(NotesDeleteZone)));

    const btn = document.getElementById('btn-delete');
    expect(btn).toBeTruthy();
    expect(btn.getAttribute('title')).not.toMatch(/raw|digest/i);
    expect(btn.textContent).toContain('Delete this entry');
  });

  it('drops the delete button stroke and keeps the zone divider', () => {
    const css = readFileSync(
      join(dirname(fileURLToPath(import.meta.url)), '../../frontend/app.css'),
      'utf8',
    );
    expect(css).toMatch(/\.md-body-delete-zone button\s*\{[^}]*border:\s*none/);
    expect(css).toMatch(/\.md-body-delete-zone button\s*\{[^}]*background:\s*var\(--bg-muted\)/);
    expect(css).toMatch(/\[data-theme="light"\] \.md-body-delete-zone button\s*\{[^}]*background:\s*var\(--bg-hex-fff0f0\)/);
    expect(css).toMatch(/\.md-body-delete-zone button\s*\{[^}]*min-height:\s*32px/);
    expect(css).toMatch(/\.md-body-delete-icon\s*\{[^}]*height:\s*1em/);
    expect(css).toMatch(/\.md-body-delete-icon\s*\{[^}]*width:\s*1em/);
    expect(css).toMatch(/\.md-body-delete-glyph\s*\{[^}]*scale:\s*0\.8/);
    expect(css).toMatch(/\.md-body-delete-zone\s*\{[^}]*border-top:\s*1px solid var\(--border-muted\)/);
    expect(css).not.toMatch(
      /\.md-body-delete-zone button\s*\{[^}]*border:\s*1px solid var\(--border-hex-ffb8b8\)/,
    );
  });
});
