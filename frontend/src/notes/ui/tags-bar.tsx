import { useEffect, useState, useSyncExternalStore } from 'react';
import { createPortal } from 'react-dom';
import { notifyState, useHostState } from '../state/host.ts';
import { TAG_VALUE_MAX_LEN } from '../../host/constants.ts';
import { suggestTags } from '../commands/tag-suggest.ts';
import {
  attachNoteTag,
  detachNoteTag,
  findTagKeyByValue,
  refreshTagDisplayGlobally,
  updateNoteTagValue,
} from '../commands/tags-bar.ts';
import { tagsPopoverStore } from '../state/tags-popover.ts';
import { ViewerHeaderIcon } from '../../shared/viewer-header-icons.tsx';

export { refreshTagDisplayGlobally };

/** Tests / leftover callers: refresh the React bar. Production reads useHostState. */
export function renderTagsBar() {
  notifyState();
}

let popoverAnchor: HTMLElement | null = null;

/** The popover is fixed and right-aligned with the toolbar button that opened it. */
export function openTagsPopover(anchor: HTMLElement) {
  popoverAnchor = anchor;
  const rect = anchor.getBoundingClientRect();
  tagsPopoverStore.set({
    open: true,
    top: rect.bottom + 6,
    right: Math.max(8, Math.round(window.innerWidth - rect.right)),
  });
}

export function closeTagsPopover(refocusEntry = false) {
  if (!tagsPopoverStore.getSnapshot().open) return;
  tagsPopoverStore.set({ open: false, top: 0, right: 0 });
  popoverAnchor = null;
  if (refocusEntry) document.getElementById('btn-doc-tags')?.focus();
}

function repositionTagsPopover() {
  if (!popoverAnchor) return;
  const rect = popoverAnchor.getBoundingClientRect();
  tagsPopoverStore.set({
    open: true,
    top: rect.bottom + 6,
    right: Math.max(8, Math.round(window.innerWidth - rect.right)),
  });
}

/** Toolbar entry: the outlined tag icon. It replaces the tags bar row under the header. */
export function TagsToolbarButton() {
  const host = useHostState();
  const entry = host.viewer.entry;
  const creating = Boolean(host.viewer.createSession);
  const snap = useSyncExternalStore(tagsPopoverStore.subscribe, tagsPopoverStore.getSnapshot);
  const visible = Boolean(entry) && !creating;
  return (
    <button
      className="md-header-btn viewer-chrome-persisted"
      id="btn-doc-tags"
      type="button"
      aria-label="Tags"
      aria-haspopup="dialog"
      aria-expanded={snap.open ? 'true' : 'false'}
      data-tip="Tags"
      style={{ display: visible ? undefined : 'none' }}
      onClick={(event) => {
        if (snap.open) closeTagsPopover();
        else openTagsPopover(event.currentTarget);
      }}
    >
      <ViewerHeaderIcon name="tag" />
    </button>
  );
}

export function NotesTagsBar() {
  const host = useHostState();
  const snap = useSyncExternalStore(tagsPopoverStore.subscribe, tagsPopoverStore.getSnapshot);
  const entry = host.viewer.entry;
  const creating = Boolean(host.viewer.createSession);
  const open = snap.open && Boolean(entry) && !creating;
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  const [addValue, setAddValue] = useState('');
  const [addPreview, setAddPreview] = useState('');
  const [addPreviewColor, setAddPreviewColor] = useState('#8c959f');
  const [editValue, setEditValue] = useState('');
  const [editError, setEditError] = useState('');
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setEditingIndex(null);
    setAddValue('');
    setAddPreview('');
    setEditValue('');
    setEditError('');
    closeTagsPopover();
  }, [entry?._id || entry?.id]);

  // A closed popover reopens clean: no half-finished edit or draft left behind.
  useEffect(() => {
    if (open) return;
    setEditingIndex(null);
    setAddValue('');
    setAddPreview('');
    setEditValue('');
    setEditError('');
  }, [open]);

  useEffect(() => {
    if (!open) return undefined;
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      event.preventDefault();
      closeTagsPopover(true);
    };
    const onPointerDown = (event: PointerEvent) => {
      const el = event.target instanceof HTMLElement ? event.target : null;
      if (!el || el.closest('[data-role="tags-popover"], #btn-doc-tags')) return;
      closeTagsPopover();
    };
    // Cross-origin iframes (embedded Sketch) swallow pointer events, but stealing
    // focus from the window still fires blur; close without touching the iframe.
    const onBlur = () => closeTagsPopover();
    const onResize = () => repositionTagsPopover();
    window.addEventListener('keydown', onKey);
    document.addEventListener('pointerdown', onPointerDown);
    window.addEventListener('blur', onBlur);
    window.addEventListener('resize', onResize);
    return () => {
      window.removeEventListener('keydown', onKey);
      document.removeEventListener('pointerdown', onPointerDown);
      window.removeEventListener('blur', onBlur);
      window.removeEventListener('resize', onResize);
    };
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const popover = document.querySelector('[data-role="tags-popover"]');
    popover?.querySelector<HTMLElement>('.md-tag-entry-row input')?.focus();
  }, [open]);

  if (!open) return null;

  const tags = entry?.tags || [];
  const suggestions = suggestTags(addValue, host.index.tagsRegistry);

  async function onDetach(key?: string) {
    if (!key) return;
    const result = await detachNoteTag(key);
    if (!result.ok) alert(result.error || 'Failed to remove tag');
  }

  async function onSaveEdit(key?: string) {
    setSaving(true);
    setEditError('');
    try {
      if (!key) return;
      const result = await updateNoteTagValue(key, editValue);
      if (!result.ok) {
        setEditError(result.error || 'Update failed');
        return;
      }
      setEditingIndex(null);
    } catch (e) {
      setEditError(e instanceof Error ? e.message : 'Update failed');
    } finally {
      setSaving(false);
    }
  }

  async function onAttach(payload: { key?: string; value?: string }) {
    const result = await attachNoteTag(payload);
    if (!result.ok) {
      setAddPreview(`Error: ${result.error || 'Add failed'}`);
      setAddPreviewColor('#cf222e');
      return;
    }
    setAddValue('');
    setAddPreview(result.idempotent ? 'Tag added' : '');
    setAddPreviewColor('#656d76');
    document.querySelector<HTMLInputElement>('.md-tag-entry-row input')?.focus();
  }

  function onAddConfirm() {
    const trimmed = addValue.trim();
    if (!trimmed) return;
    const existingKey = findTagKeyByValue(host.index.tagsRegistry, trimmed);
    void onAttach(existingKey ? { key: existingKey } : { value: trimmed });
  }

  return createPortal(
    <div
      id="md-tags-bar"
      data-role="tags-popover"
      role="dialog"
      aria-label="Tags"
      className="tags-popover viewer-chrome-persisted"
      style={{ top: snap.top, right: snap.right }}
    >
      <div className="tags-popover-title">Tags</div>
      {tags.map((tag, i) => (
        <span
          key={`${tag.key || tag.value || 'u'}-${i}`}
          className="md-tag-chip-wrap"
          data-tag-index={i}
          style={{ display: 'inline-flex', alignItems: 'center', gap: 4, marginRight: 8, flexShrink: 0 }}
        >
          {editingIndex === i ? (
            <>
              <span className="md-tag-pill-editor">
                <span aria-hidden="true">🏷</span>
                <input
                  type="text"
                  aria-label="Edit tag"
                  value={editValue}
                  maxLength={TAG_VALUE_MAX_LEN}
                  disabled={saving}
                  onChange={(e) => setEditValue(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                      e.preventDefault();
                      void onSaveEdit(tag.key);
                    }
                    if (e.key === 'Escape') {
                      e.preventDefault();
                      e.stopPropagation();
                      setEditingIndex(null);
                      setEditError('');
                    }
                  }}
                  style={{ width: `${Math.min(22, Math.max(4, editValue.length + 1))}ch` }}
                  autoFocus
                />
              </span>
              <button
                type="button"
                className="md-tag-chip-del"
                data-tip="Remove this tag"
                aria-label="Remove this tag"
                data-action="detach"
                disabled={saving}
                onClick={() => void onDetach(tag.key)}
              >
                ×
              </button>
              {editError ? (
                <span data-role="err" style={{ fontSize: 11, color: '#cf222e' }}>
                  {editError}
                </span>
              ) : null}
            </>
          ) : tag.unknown ? (
            <span className="md-tag-chip tag-unknown">{`🏷 ${tag.value || tag.key || 'unknown'}`}</span>
          ) : (
            <>
              <button
                type="button"
                className="md-tag-chip"
                data-tip="Edit tag text"
                aria-label={`Edit ${tag.value || tag.key}`}
                data-action="edit"
                onClick={() => {
                  setEditingIndex(i);
                  setEditValue(tag.value || '');
                  setEditError('');
                }}
              >
                {`🏷 ${tag.value || tag.key}`}
              </button>
              <button
                type="button"
                className="md-tag-chip-del"
                data-tip="Remove this tag"
                aria-label="Remove this tag"
                data-action="detach"
                onClick={() => void onDetach(tag.key)}
              >
                ×
              </button>
            </>
          )}
        </span>
      ))}
      <div className="md-tag-entry-row">
        <div style={{ position: 'relative', display: 'flex', flexDirection: 'column', gap: 4, flex: 1, minWidth: 0 }}>
          <input
            type="text"
            aria-label="Tag text"
            placeholder="Tag text…"
            maxLength={TAG_VALUE_MAX_LEN}
            value={addValue}
            onChange={(e) => {
              setAddValue(e.target.value);
              setAddPreview('');
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                onAddConfirm();
              }
            }}
            style={{ width: '100%' }}
          />
          <div
            className="md-tag-suggest-list"
            style={{
              display: suggestions.length ? 'block' : 'none',
              position: 'absolute',
              top: '100%',
              zIndex: 10,
              background: '#fff',
              border: '1px solid #d0d7de',
              borderRadius: 6,
              boxShadow: '0 4px 12px rgba(0,0,0,.12)',
              maxHeight: 160,
              overflow: 'auto',
            }}
          >
            {suggestions.map((item) => (
              <button
                key={item.key}
                type="button"
                className="md-tag-suggest-item"
                onClick={() => void onAttach({ key: item.key })}
              >
                {item.value}
              </button>
            ))}
          </div>
        </div>
        <button
          type="button"
          className="md-tag-entry-add"
          data-action="ok"
          onClick={onAddConfirm}
        >
          Add
        </button>
        <button
          type="button"
          className="md-tag-entry-cancel"
          data-action="cancel"
          onClick={() => closeTagsPopover(true)}
        >
          Cancel
        </button>
      </div>
      {addPreview ? (
        <span data-role="preview" style={{ fontSize: 11, color: addPreviewColor, width: '100%' }}>
          {addPreview}
        </span>
      ) : null}
    </div>,
    document.body,
  );
}
