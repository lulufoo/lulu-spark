import { useSyncExternalStore } from 'react';
import {
  addDocCategory,
  removeDocCategory,
  saveCategoryLabel,
  setCategoryLabelDraft,
  setNewCategoryExposed,
  setNewCategoryId,
  setNewCategoryLabel,
  toggleCategoryExposed,
} from '../../commands/settings/document-categories.ts';
import { documentCategoriesStore } from '../../state/settings/document-categories.ts';

export function DocumentPanel() {
  const snap = useSyncExternalStore(
    documentCategoriesStore.subscribe,
    documentCategoriesStore.getSnapshot,
  );

  return (
    <div id="settings-panel-document" className="settings-panel">
      <p className="settings-field-hint">
        These labels classify documents. Exposed items appear in MCP{' '}
        <code>list_doc_categories</code> and are accepted by <code>create_doc</code>. An id
        cannot change after it is added.
      </p>
      <div id="doc-cat-list">
        {snap.rows.map((row) => (
          <div key={row.id} className="doc-cat-row" data-doc-cat={row.id}>
            <code className="doc-cat-id" data-doc-cat-id={row.id}>
              {row.id}
            </code>
            <input
              type="text"
              className="doc-cat-label"
              data-doc-cat-label={row.id}
              value={row.label}
              spellCheck={false}
              autoComplete="off"
              aria-label={`Label for ${row.id}`}
              disabled={snap.busy}
              onChange={(e) => setCategoryLabelDraft(row.id, e.target.value)}
              onBlur={() => {
                void saveCategoryLabel(row.id);
              }}
            />
            <label className="doc-cat-exposed">
              <input
                type="checkbox"
                data-doc-cat-exposed={row.id}
                checked={row.exposed}
                disabled={snap.busy}
                onChange={(e) => void toggleCategoryExposed(row.id, e.target.checked)}
              />
              MCP
            </label>
            <button
              type="button"
              className="doc-cat-del"
              data-doc-cat-remove={row.id}
              data-tip="Remove this category"
              aria-label={`Remove ${row.id}`}
              disabled={snap.busy}
              onClick={() => void removeDocCategory(row.id)}
            >
              ×
            </button>
          </div>
        ))}
      </div>
      <div className="doc-cat-add">
        <input
          id="doc-cat-new-id"
          type="text"
          placeholder="id"
          value={snap.newId}
          spellCheck={false}
          autoComplete="off"
          disabled={snap.busy}
          onChange={(e) => setNewCategoryId(e.target.value)}
        />
        <input
          id="doc-cat-new-label"
          type="text"
          placeholder="Label"
          value={snap.newLabel}
          spellCheck={false}
          autoComplete="off"
          disabled={snap.busy}
          onChange={(e) => setNewCategoryLabel(e.target.value)}
        />
        <label className="doc-cat-exposed">
          <input
            id="doc-cat-new-exposed"
            type="checkbox"
            checked={snap.newExposed}
            disabled={snap.busy}
            onChange={(e) => setNewCategoryExposed(e.target.checked)}
          />
          MCP
        </label>
        <button
          id="btn-doc-cat-add"
          type="button"
          className="btn-settings-save"
          disabled={snap.busy || !snap.newId.trim() || !snap.newLabel.trim()}
          onClick={() => void addDocCategory()}
        >
          Add
        </button>
      </div>
      <div id="settings-result-document" className="settings-result">
        {snap.error}
      </div>
    </div>
  );
}
