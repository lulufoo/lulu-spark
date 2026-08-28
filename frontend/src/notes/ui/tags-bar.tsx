import { useEffect, useState, type CSSProperties } from 'react';
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

export { refreshTagDisplayGlobally };

const barStyle: CSSProperties = {
  padding: '8px 20px',
  borderBottom: '1px solid #d0d7de',
  flexWrap: 'wrap',
  alignItems: 'center',
  gap: '4px 0',
};

/** Tests / leftover callers: refresh the React bar. Production reads useHostState. */
export function renderTagsBar() {
  notifyState();
}

export function NotesTagsBar() {
  const host = useHostState();
  const entry = host.viewer.entry;
  const creating = Boolean(host.viewer.createSession);
  const tags = entry?.tags || [];
  const [adding, setAdding] = useState(false);
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  const [addValue, setAddValue] = useState('');
  const [addPreview, setAddPreview] = useState('');
  const [addPreviewColor, setAddPreviewColor] = useState('#8c959f');
  const [editValue, setEditValue] = useState('');
  const [editError, setEditError] = useState('');
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setAdding(false);
    setEditingIndex(null);
    setAddValue('');
    setAddPreview('');
    setEditValue('');
    setEditError('');
  }, [entry?.common_path]);

  if (!entry || creating) {
    return <div id="md-tags-bar" className="viewer-chrome-persisted" style={{ display: 'none', ...barStyle }} />;
  }

  const suggestions = adding ? suggestTags(addValue, host.index.tagsRegistry) : [];

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
    if (result.idempotent) {
      setAddPreview('Tag added');
      setAddPreviewColor('#656d76');
    }
    setAdding(false);
    setAddValue('');
    setAddPreview('');
  }

  function onAddConfirm() {
    const trimmed = addValue.trim();
    const existingKey = findTagKeyByValue(host.index.tagsRegistry, trimmed);
    void onAttach(existingKey ? { key: existingKey } : { value: trimmed });
  }

  return (
    <div id="md-tags-bar" className="viewer-chrome-persisted" style={{ display: 'flex', ...barStyle }}>
      {tags.map((tag, i) => (
        <span
          key={`${tag.key || tag.value || 'u'}-${i}`}
          className="md-tag-chip-wrap"
          data-tag-index={i}
          style={{ display: 'inline-flex', alignItems: 'center', gap: 4, marginRight: 10, flexShrink: 0 }}
        >
          {editingIndex === i ? (
            <span className="md-tag-edit-row" style={{ display: 'inline-flex', alignItems: 'center', gap: 4, marginRight: 10, flexShrink: 0 }}>
              <input
                type="text"
                value={editValue}
                maxLength={TAG_VALUE_MAX_LEN}
                onChange={(e) => setEditValue(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    void onSaveEdit(tag.key);
                  }
                  if (e.key === 'Escape') {
                    e.preventDefault();
                    setEditingIndex(null);
                  }
                }}
                style={{ fontSize: 12, padding: '2px 6px', border: '1px solid #e8c547', borderRadius: 4, minWidth: 120 }}
                autoFocus
              />
              <button
                type="button"
                className="md-header-btn primary"
                data-action="ok"
                style={{ fontSize: 11, padding: '2px 8px' }}
                disabled={saving}
                onClick={() => void onSaveEdit(tag.key)}
              >
                Save
              </button>
              <button
                type="button"
                className="md-header-btn"
                data-action="cancel"
                style={{ fontSize: 11, padding: '2px 8px' }}
                onClick={() => setEditingIndex(null)}
              >
                Cancel
              </button>
              <span data-role="err" style={{ fontSize: 11, color: '#cf222e' }}>
                {editError}
              </span>
            </span>
          ) : tag.unknown ? (
            <span className="md-tag-chip tag-unknown">{`🏷 ${tag.value || tag.key || 'unknown'}`}</span>
          ) : (
            <>
              <button
                type="button"
                className="md-tag-chip"
                title="Edit tag text"
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
                title="Remove this tag"
                data-action="detach"
                onClick={() => void onDetach(tag.key)}
              >
                ×
              </button>
            </>
          )}
        </span>
      ))}
      {adding ? (
        <div className="md-tag-add-row" style={{ display: 'flex', flexWrap: 'wrap', alignItems: 'center', gap: 6, width: '100%', marginTop: 6 }}>
          <div style={{ position: 'relative', display: 'flex', flexDirection: 'column', gap: 4 }}>
            <input
              type="text"
              placeholder="Tag text…"
              maxLength={TAG_VALUE_MAX_LEN}
              value={addValue}
              onChange={(e) => setAddValue(e.target.value)}
              style={{ fontSize: 12, padding: '4px 8px', border: '1px solid #d0d7de', borderRadius: 4, minWidth: 160 }}
              autoFocus
            />
            <div
              className="md-tag-suggest-list"
              style={{
                display: suggestions.length ? 'block' : 'none',
                position: 'absolute',
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
          <button type="button" className="md-header-btn primary" data-action="ok" style={{ fontSize: 11, padding: '2px 8px' }} onClick={onAddConfirm}>
            Add
          </button>
          <button
            type="button"
            className="md-header-btn"
            data-action="cancel"
            style={{ fontSize: 11, padding: '2px 8px' }}
            onClick={() => {
              setAdding(false);
              setAddValue('');
              setAddPreview('');
            }}
          >
            Cancel
          </button>
          <span data-role="preview" style={{ fontSize: 11, color: addPreviewColor, width: '100%' }}>
            {addPreview}
          </span>
        </div>
      ) : (
        <button
          type="button"
          className="md-header-btn"
          data-action="add-tag"
          style={{ fontSize: 11, padding: '2px 8px', marginLeft: 'auto', flexShrink: 0 }}
          onClick={() => setAdding(true)}
        >
          ＋ Add tag
        </button>
      )}
    </div>
  );
}
