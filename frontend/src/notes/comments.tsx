// @ts-nocheck — ported from JS; state shapes stay unchecked like checkJs:false.
import { memo, useSyncExternalStore } from 'react';
import { state } from '../host/state.ts';
import * as api from '../host/api.ts';
import { reorderComments } from '../host/api.ts';
import { ensureLayerComments, swapAdjacent, validateCommentIdsForReorder } from '../shared/comment-reorder.ts';
import { nowTs } from '../shared/utils.ts';
import { openSettleDialog } from './settle-dialog.tsx';
import { renderLinksBar } from './links-bar.tsx';
import { confirmDeleteComment, removeCorpusComment } from '../shared/comment-delete.tsx';
import { pasteIntoCommentEditor, prepareCommentMarkdown, renderCommentMarkdown } from '../shared/comment-markdown.ts';
import { renderMermaidBlocks } from '../shared/mermaid-render.ts';
import { createModuleStore } from '../shared/module-store.ts';
import { renderToHtml } from '../island.ts';

const openStore = createModuleStore(false);

let _floatNavObserver: IntersectionObserver | null = null;

type CommentRec = { id: string; text: string; ts?: string };
type NoteEntry = { common_path: string; links?: { url: string }[] };
type LayerData = { comments?: CommentRec[] };
type Annotation = Record<string, LayerData | undefined> & { links?: { url: string }[] };

function annotation(): Annotation {
  return state.viewer.annotation as Annotation;
}

const _tip = () => document.getElementById('comment-preview-tip');

function _showTip(text: string, e: MouseEvent) {
  const tip = _tip();
  if (!tip) return;
  const preview = text.length > 600 ? text.slice(0, 600) + '\n\n…' : text;
  const inner = renderCommentMarkdown(preview);
  tip.innerHTML = renderToHtml(
    <div className="comment-item-text" dangerouslySetInnerHTML={{ __html: inner }} />,
  );
  tip.style.display = 'block';
  _moveTip(e);
}

function _moveTip(e: MouseEvent) {
  const tip = _tip();
  if (!tip || tip.style.display === 'none') return;
  const GAP = 12;
  let x = e.clientX + GAP;
  let y = e.clientY + GAP;
  const tw = tip.offsetWidth;
  const th = tip.offsetHeight;
  if (x + tw > window.innerWidth - 8) x = e.clientX - tw - GAP;
  if (y + th > window.innerHeight - 8) y = e.clientY - th - GAP;
  tip.style.left = x + 'px';
  tip.style.top = y + 'px';
}

function _hideTip() {
  const tip = _tip();
  if (tip) tip.style.display = 'none';
}

function FloatNavButtons({ count }: { count: number }) {
  return (
    <>
      {Array.from({ length: count }, (_, i) => (
        <button key={i} className="comment-float-btn" data-index={i} title={`Edit comment ${i + 1}`}>
          {i + 1}
        </button>
      ))}
    </>
  );
}

function updateFloatNav(comments: CommentRec[] | undefined, layer: string | null, entry: NoteEntry | null) {
  const nav = document.getElementById('comment-float-nav');
  if (!nav) return;
  nav.innerHTML = '';
  nav.style.display = 'none';
  if (_floatNavObserver) {
    _floatNavObserver.disconnect();
    _floatNavObserver = null;
  }
  if (!comments || comments.length === 0) return;

  nav.innerHTML = renderToHtml(<FloatNavButtons count={comments.length} />);
  nav.querySelectorAll('.comment-float-btn').forEach((btn) => {
    const i = Number((btn as HTMLElement).dataset.index);
    const c = comments[i];
    btn.addEventListener('click', () => openCommentDialog(c, layer, entry, i + 1));
    btn.addEventListener('mouseenter', (e) => _showTip(c.text, e as MouseEvent));
    btn.addEventListener('mousemove', (e) => _moveTip(e as MouseEvent));
    btn.addEventListener('mouseleave', () => _hideTip());
  });

  const bar = document.getElementById('md-comments-bar');
  if (!bar) return;
  const mdBody = document.getElementById('md-body');
  _floatNavObserver = new IntersectionObserver(
    ([e]) => {
      nav.style.display = e.isIntersecting ? 'none' : 'flex';
    },
    { root: mdBody, threshold: 0 },
  );
  _floatNavObserver.observe(bar);
}

function commentTs(c: CommentRec) {
  if (c.ts && c.ts.length >= 12) {
    return `${c.ts.slice(0, 4)}-${c.ts.slice(4, 6)}-${c.ts.slice(6, 8)} ${c.ts.slice(8, 10)}:${c.ts.slice(10, 12)}`;
  }
  return '';
}

function CommentItemView({
  c,
  noteIndex,
  showUp,
  showDown,
}: {
  c: CommentRec;
  noteIndex: number;
  showUp: boolean;
  showDown: boolean;
}) {
  const useMarkdown = typeof marked !== 'undefined' && typeof marked?.parse === 'function';
  return (
    <div className="comment-item" data-cid={c.id}>
      <button type="button" className="comment-item-del-x" title="Delete" data-action="delete">
        ×
      </button>
      <span className="comment-item-prefix">{`NOTE ${noteIndex}:`}</span>
      {useMarkdown ? (
        <div className="comment-item-text" dangerouslySetInnerHTML={{ __html: renderCommentMarkdown(c.text) }} />
      ) : (
        <div className="comment-item-text">{c.text}</div>
      )}
      <span className="comment-item-ts">{commentTs(c)}</span>
      <div className="comment-item-actions">
        <button type="button" className="comment-item-action-btn" title="Promote to knowledge repo" data-action="settle">
          ⬆ Promote
        </button>
        <button type="button" className="comment-item-action-btn" data-action="edit">
          Edit
        </button>
        {showUp ? (
          <button type="button" className="comment-item-action-btn comment-item-order-btn" title="Move up" data-action="up">
            ↑
          </button>
        ) : null}
        {showDown ? (
          <button type="button" className="comment-item-action-btn comment-item-order-btn" title="Move down" data-action="down">
            ↓
          </button>
        ) : null}
      </div>
    </div>
  );
}

function CommentsBar({ comments }: { comments: CommentRec[] }) {
  return (
    <>
      <div className="comment-bar-header">
        <span>{`💬 Comment · ${comments.length}`}</span>
      </div>
      {comments.map((c, i) => (
        <CommentItemView
          key={c.id}
          c={c}
          noteIndex={i + 1}
          showUp={i > 0}
          showDown={i < comments.length - 1}
        />
      ))}
    </>
  );
}

export function renderComments(rawAnnotation: unknown, layer: string, entry: NoteEntry | null) {
  const existing = document.getElementById('md-comments-bar');
  if (existing) existing.remove();
  if (!entry) {
    updateFloatNav([], null, null);
    return;
  }

  const annotationMap = (rawAnnotation || {}) as Annotation;
  const layerData = annotationMap[layer] || {};
  const comments = layerData.comments || [];
  if (comments.length === 0) {
    updateFloatNav([], layer, entry);
    return;
  }

  const body = document.getElementById('md-body');
  if (!body) return;
  const bar = document.createElement('div');
  bar.id = 'md-comments-bar';
  bar.className = 'md-comments-bar';
  bar.innerHTML = renderToHtml(<CommentsBar comments={comments} />);
  bindCommentBar(bar, comments, layer, entry);
  body.insertBefore(bar, body.firstChild);
  updateFloatNav(comments, layer, entry);
}

function bindCommentBar(bar: HTMLElement, comments: CommentRec[], layer: string, entry: NoteEntry) {
  bar.querySelectorAll('.comment-item').forEach((item) => {
    const cid = (item as HTMLElement).dataset.cid;
    const c = comments.find((x) => x.id === cid);
    if (!c) return;
    const noteIndex = comments.indexOf(c) + 1;
    const idx = noteIndex - 1;
    item.querySelector('[data-action="delete"]')?.addEventListener('click', async () => {
      if (!(await confirmDeleteComment())) return;
      try {
        await removeCorpusComment(c, layer, entry);
        renderComments(state.viewer.annotation, layer, entry);
      } catch (e) {
        alert(`Delete failed: ${(e as Error).message}`);
      }
    });
    item.querySelector('[data-action="settle"]')?.addEventListener('click', () => openSettleDialog(c, layer, entry));
    item.querySelector('[data-action="edit"]')?.addEventListener('click', () => openCommentDialog(c, layer, entry, noteIndex));
    item.querySelector('[data-action="up"]')?.addEventListener('click', () => moveCorpusComment(layer, entry, idx, -1));
    item.querySelector('[data-action="down"]')?.addEventListener('click', () => moveCorpusComment(layer, entry, idx, 1));
  });
}

async function moveCorpusComment(layer: string, entry: NoteEntry, idx: number, delta: number) {
  const arr = ensureLayerComments(state.viewer.annotation, layer);
  if (!swapAdjacent(arr, idx, delta)) return;
  renderComments(state.viewer.annotation, layer, entry);
  const check = validateCommentIdsForReorder(arr);
  if (!check.ok) {
    alert(check.error);
    return;
  }
  try {
    const data = await reorderComments(entry.common_path, layer, check.ids);
    if (data?.ok !== true) throw new Error(data?.error || 'failed');
  } catch (e) {
    alert(`Reorder failed: ${(e as Error).message}`);
  }
}

type CommentEditCtx = {
  c?: CommentRec;
  layer?: string | null;
  entry?: NoteEntry | null;
  noteIndex?: number;
};

let _commentEditCtx: CommentEditCtx | null = null;
let _draftKey: string | null = null;
let _draftSaveTimer: ReturnType<typeof setTimeout> | null = null;
let _previewModeText: string | null = null;

function _scheduleDraftSave() {
  if (!_draftKey) return;
  if (_draftSaveTimer) clearTimeout(_draftSaveTimer);
  _draftSaveTimer = setTimeout(() => {
    const text = prepareCommentMarkdown(document.getElementById('comment-dialog-content')!.innerText);
    api.saveDraft(_draftKey, text).catch(() => {});
  }, 800);
}

function _clearDraft() {
  if (!_draftKey) return;
  if (_draftSaveTimer) clearTimeout(_draftSaveTimer);
  api.saveDraft(_draftKey, '').catch(() => {});
  _draftKey = null;
}

export async function openCommentDialog(
  editComment: CommentRec | null,
  layer: string | null,
  entry: NoteEntry | null,
  noteIndex: number,
) {
  if (!state.viewer.entry && !entry) return;
  const titleEl = document.getElementById('comment-dialog-title')!;
  const content = document.getElementById('comment-dialog-content')!;
  if (editComment) {
    _commentEditCtx = { c: editComment, layer, entry, noteIndex };
    _draftKey = null;
    titleEl.textContent = '💬 Edit comment';
    content.innerText = editComment.text;
  } else {
    _commentEditCtx = { noteIndex };
    const commonPath = (entry || state.viewer.entry).common_path;
    _draftKey = commonPath;
    titleEl.textContent = '💬 Add comment';
    content.innerText = '';
    try {
      const draft = await api.getDraft(commonPath);
      if (draft.content) content.innerText = draft.content;
    } catch {
      /* ignore */
    }
  }
  openStore.set(true);
  document.getElementById('comment-dialog')?.classList.add('open');
  _resetDialogTabs();
  requestAnimationFrame(() => {
    content.focus();
    const range = document.createRange();
    const sel = window.getSelection();
    range.selectNodeContents(content);
    range.collapse(false);
    sel?.removeAllRanges();
    sel?.addRange(range);
  });
}

export function closeCommentDialog() {
  openStore.set(false);
  document.getElementById('comment-dialog')?.classList.remove('open');
  _commentEditCtx = null;
  _clearDraft();
}

export async function saveComment() {
  if (!_commentEditCtx) return;
  const content = document.getElementById('comment-dialog-content')!;
  const text = prepareCommentMarkdown((_previewModeText !== null ? _previewModeText : content.innerText).trim());
  if (!text) return;
  const saveBtn = document.getElementById('btn-comment-save') as HTMLButtonElement;
  saveBtn.disabled = true;
  saveBtn.textContent = 'Saving…';
  const ts = nowTs();
  try {
    if (_commentEditCtx.c) {
      const { c, layer, entry } = _commentEditCtx;
      if (text === c.text) {
        closeCommentDialog();
        return;
      }
      const data = await api.updateComments(entry!.common_path, layer, { id: c.id, text }, ts);
      if (data.ok) {
        c.text = text;
        const ld = annotation()[layer as string] || {};
        const idx = (ld.comments || []).findIndex((x) => x.id === c.id);
        if (idx >= 0) ld.comments![idx].text = text;
        closeCommentDialog();
        renderComments(state.viewer.annotation, layer as string, entry!);
      } else {
        alert(`Save failed: ${data.error}`);
      }
    } else {
      if (!state.viewer.entry) return;
      const data = await api.updateComments(state.viewer.entry.common_path, state.viewer.layer, { text }, ts);
      if (data.ok) {
        if (!state.viewer.annotation[state.viewer.layer]) state.viewer.annotation[state.viewer.layer] = {};
        if (!state.viewer.annotation[state.viewer.layer].comments) state.viewer.annotation[state.viewer.layer].comments = [];
        state.viewer.annotation[state.viewer.layer].comments.push({ id: data.id, text, ts });
        _clearDraft();
        closeCommentDialog();
        renderComments(state.viewer.annotation, state.viewer.layer, state.viewer.entry);
      } else {
        alert(`AddFailed: ${data.error}`);
      }
    }
  } catch (e) {
    alert(`Save failed: ${(e as Error).message}`);
  } finally {
    saveBtn.disabled = false;
    saveBtn.textContent = 'Save';
  }
}

async function onNoteCommentTabClick(btn: HTMLElement) {
  document.querySelectorAll('#comment-dialog .comment-tab-btn').forEach((b) => b.classList.remove('active'));
  btn.classList.add('active');
  const isPreview = btn.dataset.tab === 'preview';
  const editorBox = document.getElementById('comment-editor-box')!;
  const previewPane = document.getElementById('comment-preview-pane')!;
  if (isPreview) {
    _previewModeText = prepareCommentMarkdown(document.getElementById('comment-dialog-content')!.innerText);
    const inner = renderCommentMarkdown(_previewModeText);
    previewPane.innerHTML = renderToHtml(
      <div className="comment-item-text" dangerouslySetInnerHTML={{ __html: inner }} />,
    );
    await renderMermaidBlocks(previewPane);
    editorBox.style.display = 'none';
    previewPane.style.display = 'block';
  } else {
    _previewModeText = null;
    editorBox.style.display = '';
    previewPane.style.display = 'none';
  }
}

function _resetDialogTabs() {
  _previewModeText = null;
  document.querySelectorAll('#comment-dialog .comment-tab-btn').forEach((b) => {
    b.classList.toggle('active', (b as HTMLElement).dataset.tab === 'edit');
  });
  const editorBox = document.getElementById('comment-editor-box');
  const previewPane = document.getElementById('comment-preview-pane');
  if (editorBox) editorBox.style.display = '';
  if (previewPane) previewPane.style.display = 'none';
}

const _btnAddComment = document.getElementById('btn-add-comment');
if (_btnAddComment) {
  _btnAddComment.addEventListener('click', () => {
    const layerData = (state.viewer.annotation && state.viewer.annotation[state.viewer.layer]) || {};
    const nextIdx = (layerData.comments || []).length + 1;
    openCommentDialog(null, null, null, nextIdx);
  });
}

function onNoteCommentContentKeyDown(e: KeyboardEvent) {
  if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
    e.preventDefault();
    void saveComment();
  }
}

function onNoteCommentContentPaste(e: ClipboardEvent) {
  e.preventDefault();
  const el = document.getElementById('comment-dialog-content')!;
  pasteIntoCommentEditor(el, e.clipboardData);
  const sel = window.getSelection();
  if (sel && sel.rangeCount > 0) {
    const range = sel.getRangeAt(0);
    const rect = range.getBoundingClientRect();
    const elRect = el.getBoundingClientRect();
    if (rect.bottom > elRect.bottom) {
      el.scrollTop += rect.bottom - elRect.bottom + 8;
    }
  }
}

let _mouseDownOnOverlay = false;

document.addEventListener('settle:done', (event) => {
  const { commentId, layer, entry, url } = (event as CustomEvent).detail;
  const ld = annotation()[layer] || {};
  ld.comments = (ld.comments || []).filter((x) => x.id !== commentId);
  if (!ld.comments.length) delete state.viewer.annotation[layer];
  else state.viewer.annotation[layer] = ld;
  if (!state.viewer.annotation.links) state.viewer.annotation.links = [];
  state.viewer.annotation.links.push({ url });
  entry.links = state.viewer.annotation.links;
  renderComments(state.viewer.annotation, layer, entry);
  renderLinksBar(entry);
});

const NoteCommentDialogInner = memo(function NoteCommentDialogInner() {
  return (
    <div id="comment-dialog-box">
      <div id="comment-dialog-header">
        <h3 id="comment-dialog-title">💬 Add comment</h3>
        <div id="comment-dialog-tabs">
          <button
            type="button"
            className="comment-tab-btn active"
            data-tab="edit"
            onClick={(e) => void onNoteCommentTabClick(e.currentTarget)}
          >
            Edit
          </button>
          <button
            type="button"
            className="comment-tab-btn"
            data-tab="preview"
            onClick={(e) => void onNoteCommentTabClick(e.currentTarget)}
          >
            Preview
          </button>
        </div>
      </div>
      <div
        id="comment-editor-box"
        className="comment-editor-box"
        onClick={() => document.getElementById('comment-dialog-content')?.focus()}
      >
        <div
          id="comment-dialog-content"
          className="comment-editor-content"
          contentEditable={true}
          suppressContentEditableWarning={true}
          spellCheck={false}
          autoCorrect="off"
          autoCapitalize="off"
          data-placeholder="Comment… (Ctrl/Cmd+Enter to save)"
          onKeyDown={(e) => onNoteCommentContentKeyDown(e.nativeEvent)}
          onInput={() => _scheduleDraftSave()}
          onPaste={(e) => onNoteCommentContentPaste(e.nativeEvent)}
        />
      </div>
      <div id="comment-preview-pane" className="md-body" style={{ display: 'none' }} />
      <div id="comment-dialog-actions">
        <button id="btn-comment-cancel" type="button" className="md-header-btn" onClick={() => closeCommentDialog()}>
          Cancel
        </button>
        <button
          id="btn-comment-save"
          type="button"
          className="md-header-btn primary"
          onClick={() => void saveComment()}
        >
          Save
        </button>
      </div>
    </div>
  );
});

export function NoteCommentDialog() {
  const open = useSyncExternalStore(openStore.subscribe, openStore.getSnapshot);
  return (
    <div
      id="comment-dialog"
      className={open ? 'open' : undefined}
      onKeyDown={(e) => {
        if (e.key === 'Escape') {
          e.stopPropagation();
          closeCommentDialog();
        }
      }}
      onMouseDown={(e) => {
        _mouseDownOnOverlay = e.target === e.currentTarget;
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget && _mouseDownOnOverlay) {
          closeCommentDialog();
        }
      }}
    >
      <NoteCommentDialogInner />
    </div>
  );
}
