import { memo, useEffect, useState, useSyncExternalStore } from 'react';
import { notifyState, state, useHostState } from '../state/host.ts';
import * as api from '../../host/api.ts';
import { reorderKbComments } from '../../host/api.ts';
import {
  ensureKbComments,
  swapAdjacent,
  validateCommentIdsForReorder,
} from '../../shared/comment-reorder.ts';
import { nowTs } from '../../shared/utils.ts';
import { confirmDeleteComment, removeKbComment } from '../../shared/comment-delete.tsx';
import {
  pasteIntoCommentEditor,
  prepareCommentMarkdown,
  renderCommentMarkdown,
} from '../../shared/comment-markdown.ts';
import { createModuleStore } from '../../shared/module-store.ts';
import { renderMermaidBlocks } from '../../shared/mermaid-render.ts';
import { renderToHtml } from '../../island.ts';
import { type Root } from 'react-dom/client';
import { flushSync } from 'react-dom';

const openStore = createModuleStore(false);

type KbComment = { id: string; text?: string; ts?: string };
type KbAnnotation = { comments?: KbComment[] };
type ApiOk = { ok?: boolean; error?: string; id?: string };

let _kbCommentsRoot: HTMLElement | null = null;
const _kbCommentsCleanups: Array<() => void> = [];
let _kbCommentsBarRoot: Root | null = null;
let _kbFloatNavRoot: Root | null = null;
let _kbFloatNavObserver: IntersectionObserver | null = null;

function viewerAnnotation(): KbAnnotation {
  return state.viewer.annotation as KbAnnotation;
}

function kbCommentsBodyEl(): HTMLElement | null {
  if (_kbCommentsRoot) {
    return _kbCommentsRoot.querySelector('.kb-reader-body');
  }
  return document.getElementById('kb-md-body');
}

function kbFloatNavEl(): HTMLElement | null {
  if (_kbCommentsRoot) {
    return _kbCommentsRoot.querySelector('.kb-comment-float-nav');
  }
  return document.getElementById('kb-comment-float-nav');
}

const _tip = () => document.getElementById('comment-preview-tip');

function commentMarkdownHtml(text: string) {
  return renderToHtml(
    <div
      className="comment-item-text"
      dangerouslySetInnerHTML={{ __html: renderCommentMarkdown(text) }}
    />,
  );
}

function _showKbTip(text: string, e: MouseEvent) {
  const tip = _tip();
  if (!tip) return;
  const preview = text.length > 600 ? `${text.slice(0, 600)}\n\n…` : text;
  tip.innerHTML = commentMarkdownHtml(preview);
  tip.style.display = 'block';
  _moveKbTip(e);
}

function _moveKbTip(e: MouseEvent) {
  const tip = _tip();
  if (!tip || tip.style.display === 'none') return;
  const GAP = 12;
  let x = e.clientX + GAP;
  let y = e.clientY + GAP;
  const tw = tip.offsetWidth;
  const th = tip.offsetHeight;
  if (x + tw > window.innerWidth - 8) x = e.clientX - tw - GAP;
  if (y + th > window.innerHeight - 8) y = e.clientY - th - GAP;
  tip.style.left = `${x}px`;
  tip.style.top = `${y}px`;
}

function _hideKbTip() {
  const tip = _tip();
  if (tip) tip.style.display = 'none';
}

function formatCommentTs(ts: string) {
  if (ts.length < 12) return '';
  return `${ts.slice(0, 4)}-${ts.slice(4, 6)}-${ts.slice(6, 8)} ${ts.slice(8, 10)}:${ts.slice(10, 12)}`;
}

function KbCommentItem({ comment, index, comments }: { comment: KbComment; index: number; comments: KbComment[] }) {
  const ts = comment.ts || '';
  const html = typeof marked !== 'undefined' ? renderCommentMarkdown(comment.text || '') : '';
  return (
    <div className="comment-item" data-cid={comment.id} data-index={index}>
      <button type="button" className="comment-item-del-x" title="Delete" data-kb-action="delete">
        ×
      </button>
      <span className="comment-item-prefix">{`NOTE ${index + 1}:`}</span>
      {typeof marked !== 'undefined' ? (
        <div className="comment-item-text" dangerouslySetInnerHTML={{ __html: html }} />
      ) : (
        <div className="comment-item-text">{comment.text || ''}</div>
      )}
      <span className="comment-item-ts">{formatCommentTs(ts)}</span>
      <div className="comment-item-actions">
        <button type="button" className="comment-item-action-btn" data-kb-action="edit">
          Edit
        </button>
        {index > 0 ? (
          <button
            type="button"
            className="comment-item-action-btn comment-item-order-btn"
            title="Move up"
            data-kb-action="up"
          >
            ↑
          </button>
        ) : null}
        {index < comments.length - 1 ? (
          <button
            type="button"
            className="comment-item-action-btn comment-item-order-btn"
            title="Move down"
            data-kb-action="down"
          >
            ↓
          </button>
        ) : null}
      </div>
    </div>
  );
}

function KbCommentsItems({ comments }: { comments: KbComment[] }) {
  return (
    <>
      {comments.map((comment, index) => (
        <KbCommentItem key={comment.id} comment={comment} index={index} comments={comments} />
      ))}
    </>
  );
}

function KbFloatButtons({ comments }: { comments: KbComment[] }) {
  return (
    <>
      {comments.map((comment, index) => (
        <button
          key={comment.id}
          type="button"
          className="comment-float-btn"
          data-index={index}
          title={`Edit comment ${index + 1}`}
        >
          {index + 1}
        </button>
      ))}
    </>
  );
}

export function KbCommentsBar() {
  const host = useHostState();
  const comments = ((host.viewer.annotation as KbAnnotation | undefined)?.comments || []) as KbComment[];
  if (!comments.length) return null;
  return (
    <div
      id="kb-md-comments-bar"
      className="md-comments-bar"
      data-react="1"
      onClick={(e) => {
        const btn = (e.target as Element | null)?.closest?.('[data-kb-action]') as HTMLElement | null;
        if (!btn) return;
        const item = btn.closest('.comment-item') as HTMLElement | null;
        const index = Number(item?.dataset.index ?? btn.dataset.index);
        const comment = comments[index];
        if (!comment) return;
        const action = btn.dataset.kbAction;
        if (action === 'edit') openKbCommentDialog(comment, index);
        else if (action === 'delete') {
          void (async () => {
            if (!(await confirmDeleteComment())) return;
            try {
              await removeKbComment(comment);
              notifyState();
            } catch (err) {
              alert(`Delete failed: ${err instanceof Error ? err.message : String(err)}`);
            }
          })();
        } else if (action === 'up') void moveKbComment(index, -1);
        else if (action === 'down') void moveKbComment(index, 1);
      }}
    >
      <KbCommentsItems comments={comments} />
    </div>
  );
}

export function KbCommentFloatNav() {
  const host = useHostState();
  const comments = ((host.viewer.annotation as KbAnnotation | undefined)?.comments || []) as KbComment[];
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const bar = document.getElementById('kb-md-comments-bar');
    const root = document.querySelector('.kb-reader .viewer-body');
    if (!bar || !root || !comments.length) {
      setVisible(false);
      return undefined;
    }
    const observer = new IntersectionObserver(([entry]) => setVisible(!entry.isIntersecting), {
      root,
      threshold: 0,
    });
    observer.observe(bar);
    return () => observer.disconnect();
  }, [comments.length, host.viewer.kbPath]);

  const show = Boolean(comments.length && visible);
  return (
    <div
      className="kb-comment-float-nav"
      id="kb-comment-float-nav"
      data-react="1"
      style={{ display: show ? 'flex' : 'none' }}
      onClick={(e) => {
        const btn = (e.target as Element | null)?.closest?.('.comment-float-btn') as HTMLElement | null;
        if (!btn) return;
        const index = Number(btn.dataset.index);
        const comment = comments[index];
        if (comment) openKbCommentDialog(comment, index);
      }}
      onMouseOver={(e) => {
        const btn = (e.target as Element | null)?.closest?.('.comment-float-btn') as HTMLElement | null;
        if (!btn) return;
        const index = Number(btn.dataset.index);
        const comment = comments[index];
        if (comment) _showKbTip(comment.text || '', e.nativeEvent);
      }}
      onMouseMove={(e) => _moveKbTip(e.nativeEvent)}
      onMouseOut={(e) => {
        const related = e.nativeEvent.relatedTarget as Node | null;
        if (related && e.currentTarget.contains(related)) return;
        _hideKbTip();
      }}
    >
      <KbFloatButtons comments={comments} />
    </div>
  );
}

function unmountCommentsBar() {
  if (_kbCommentsBarRoot) {
    flushSync(() => {
      _kbCommentsBarRoot?.unmount();
    });
    _kbCommentsBarRoot = null;
  }
  kbCommentsBodyEl()?.querySelector('.md-comments-bar:not([data-react])')?.remove();
}

function unmountFloatNavPaint() {
  if (_kbFloatNavRoot) {
    flushSync(() => {
      _kbFloatNavRoot?.unmount();
    });
    _kbFloatNavRoot = null;
  }
}

export function renderKbComments(_annotation?: KbAnnotation | null) {
  notifyState();
}

async function moveKbComment(index: number, delta: number) {
  const { kbRepo, kbPath } = state.viewer;
  const annotation = viewerAnnotation();
  const arr = ensureKbComments(annotation) as KbComment[];
  if (!swapAdjacent(arr, index, delta)) return;
  renderKbComments(annotation);
  const check = validateCommentIdsForReorder(arr);
  if (!check.ok) {
    alert(check.error);
    return;
  }
  try {
    const data = (await reorderKbComments(kbRepo, kbPath, check.ids)) as ApiOk;
    if (data?.ok !== true) throw new Error(data?.error || 'failed');
  } catch (err) {
    alert(`Reorder save failed: ${err instanceof Error ? err.message : String(err)}`);
  }
}

let _editingComment: KbComment | null = null;

function _resetKbDialogTabs() {
  document.querySelectorAll('#kb-comment-dialog .comment-tab-btn').forEach((b) => {
    b.classList.toggle('active', (b as HTMLElement).dataset.tab === 'edit');
  });
  const editorBox = document.getElementById('kb-comment-editor-box');
  const previewPane = document.getElementById('kb-comment-preview-pane');
  if (editorBox) editorBox.style.display = '';
  if (previewPane) previewPane.style.display = 'none';
}

export function openKbCommentDialog(editComment: KbComment | null = null, _noteIndex: number | null = null) {
  _editingComment = editComment;

  const dialog = document.getElementById('kb-comment-dialog');
  const title = document.getElementById('kb-comment-dialog-title');
  const content = document.getElementById('kb-comment-dialog-content');
  if (!dialog || !content) return;

  if (title) title.textContent = editComment ? '💬 Edit comment' : '💬 Add comment';
  content.textContent = editComment?.text || '';
  openStore.set(true);
  dialog.classList.add('open');
  dialog.style.display = 'flex';
  _resetKbDialogTabs();
  content.focus();

  const range = document.createRange();
  const sel = window.getSelection();
  range.selectNodeContents(content);
  range.collapse(false);
  sel?.removeAllRanges();
  sel?.addRange(range);
}

export function closeKbCommentDialog() {
  openStore.set(false);
  const dialog = document.getElementById('kb-comment-dialog');
  if (dialog) {
    dialog.classList.remove('open');
    dialog.style.display = 'none';
  }
  _editingComment = null;
}

export async function saveKbComment() {
  const content = document.getElementById('kb-comment-dialog-content');
  if (!content) return;
  const text = prepareCommentMarkdown((content.textContent || '').trim());
  if (!text) return;

  const { kbRepo, kbPath } = state.viewer;
  const annotation = viewerAnnotation();
  if (!kbRepo || !kbPath) return;

  try {
    if (_editingComment) {
      const data = (await api.updateKbComment(kbRepo, kbPath, { id: _editingComment.id, text }, nowTs())) as ApiOk;
      if (!data.ok) throw new Error(data.error || 'failed');
      const found = annotation.comments?.find((c) => c.id === _editingComment?.id);
      if (found) {
        found.text = text;
        found.ts = nowTs();
      }
    } else {
      const data = (await api.updateKbComment(kbRepo, kbPath, { text }, nowTs())) as ApiOk;
      if (!data.ok) throw new Error(data.error || 'failed');
      if (!annotation.comments) annotation.comments = [];
      annotation.comments.push({ id: String(data.id), text, ts: nowTs() });
    }
    closeKbCommentDialog();
    renderKbComments(annotation);
  } catch (err) {
    alert(`Save failed: ${err instanceof Error ? err.message : String(err)}`);
  }
}

async function onKbCommentTabClick(btn: HTMLElement) {
  document.querySelectorAll('#kb-comment-dialog .comment-tab-btn').forEach((b) => b.classList.remove('active'));
  btn.classList.add('active');
  const isPreview = btn.dataset.tab === 'preview';
  const editorBox = document.getElementById('kb-comment-editor-box');
  const previewPane = document.getElementById('kb-comment-preview-pane');
  if (!editorBox || !previewPane) return;
  if (isPreview) {
    const text = prepareCommentMarkdown(document.getElementById('kb-comment-dialog-content')?.textContent || '');
    previewPane.innerHTML = commentMarkdownHtml(text);
    await renderMermaidBlocks(previewPane);
    editorBox.style.display = 'none';
    previewPane.style.display = 'block';
  } else {
    editorBox.style.display = '';
    previewPane.style.display = 'none';
  }
}

function onKbCommentContentKeyDown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
    e.preventDefault();
    void saveKbComment();
  }
}

function onKbCommentContentPaste(e: ClipboardEvent) {
  e.preventDefault();
  const el = document.getElementById('kb-comment-dialog-content');
  pasteIntoCommentEditor(el, e.clipboardData);
}

export function cleanupKbComments() {
  for (const fn of _kbCommentsCleanups) fn();
  _kbCommentsCleanups.length = 0;
  if (_kbFloatNavObserver) {
    _kbFloatNavObserver.disconnect();
    _kbFloatNavObserver = null;
  }
  unmountCommentsBar();
  unmountFloatNavPaint();
  const nav = kbFloatNavEl();
  if (nav && nav.dataset.react !== '1') {
    nav.innerHTML = '';
    nav.style.display = 'none';
  }
  _kbCommentsRoot = null;
}

export function initKbComments(container?: Element | null) {
  if (!container) return;
  cleanupKbComments();
  _kbCommentsRoot = container as HTMLElement;

  const addBtn = container.querySelector('.kb-btn-add-comment');
  if (addBtn) {
    const onAdd = () => openKbCommentDialog();
    addBtn.addEventListener('click', onAdd);
    _kbCommentsCleanups.push(() => addBtn.removeEventListener('click', onAdd));
  }
}

/** @deprecated use initKbComments(container) */
export function initKbCommentEvents() {
  const legacyBody = document.getElementById('kb-md-body');
  const root = legacyBody?.closest('.kb-reader') ?? legacyBody?.parentElement;
  if (root) initKbComments(root);
}

const KbCommentDialogInner = memo(function KbCommentDialogInner() {
  return (
    <div id="kb-comment-dialog-box">
      <div id="kb-comment-dialog-header">
        <h3 id="kb-comment-dialog-title">💬 Add comment</h3>
        <div id="kb-comment-dialog-tabs">
          <button
            type="button"
            className="comment-tab-btn active"
            data-tab="edit"
            onClick={(e) => void onKbCommentTabClick(e.currentTarget)}
          >
            Edit
          </button>
          <button
            type="button"
            className="comment-tab-btn"
            data-tab="preview"
            onClick={(e) => void onKbCommentTabClick(e.currentTarget)}
          >
            Preview
          </button>
        </div>
      </div>
      <div
        id="kb-comment-editor-box"
        className="comment-editor-box"
        onClick={() => document.getElementById('kb-comment-dialog-content')?.focus()}
      >
        <div
          id="kb-comment-dialog-content"
          className="comment-editor-content"
          contentEditable={true}
          suppressContentEditableWarning={true}
          spellCheck={false}
          autoCorrect="off"
          autoCapitalize="off"
          data-placeholder="Comment… (Ctrl/Cmd+Enter to save)"
          onKeyDown={(e) => onKbCommentContentKeyDown(e.nativeEvent)}
          onPaste={(e) => onKbCommentContentPaste(e.nativeEvent)}
        />
      </div>
      <div id="kb-comment-preview-pane" className="md-body" style={{ display: 'none' }} />
      <div id="kb-comment-dialog-actions">
        <button
          id="kb-btn-comment-cancel"
          type="button"
          className="md-header-btn"
          onClick={() => closeKbCommentDialog()}
        >
          Cancel
        </button>
        <button
          id="kb-btn-comment-save"
          type="button"
          className="md-header-btn primary"
          onClick={() => void saveKbComment()}
        >
          Save
        </button>
      </div>
    </div>
  );
});

export function KbCommentDialog() {
  const open = useSyncExternalStore(openStore.subscribe, openStore.getSnapshot);
  return (
    <div
      id="kb-comment-dialog"
      className={open ? 'open' : undefined}
      style={{ display: open ? 'flex' : 'none' }}
      onKeyDown={(e) => {
        if (e.key === 'Escape') {
          e.stopPropagation();
          closeKbCommentDialog();
        }
      }}
    >
      <KbCommentDialogInner />
    </div>
  );
}
