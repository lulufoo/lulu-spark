import { useSyncExternalStore } from 'react';
import {
  addDocCategory,
  removeDocCategory,
  saveCategoryLabel,
  setCategoryLabelDraft,
  setNewCategoryExposed,
  setNewCategoryLabel,
  slugFromLabel,
  toggleCategoryExposed,
} from '../../commands/settings/document-categories.ts';
import { documentCategoriesStore } from '../../state/settings/document-categories.ts';

export function DocumentPanel() {
  const snap = useSyncExternalStore(
    documentCategoriesStore.subscribe,
    documentCategoriesStore.getSnapshot,
  );
  const newId = slugFromLabel(snap.newLabel);

  return (
    <div id="settings-panel-document" className="settings-panel">
      <div className="settings-tabs" role="tablist">
        <button type="button" className="settings-tab active" data-tab="list" role="tab">
          Categories
        </button>
        <button type="button" className="settings-tab" data-tab="add" role="tab">
          Add
        </button>
      </div>
      <div
        id="settings-tab-document-list"
        className="settings-tab-panel active"
        data-tab="list"
      >
        <p className="doc-cat-lead">
          Each document has one category. Exposed categories can be chosen when creating a document.
          The id cannot change after it is added.
        </p>
        <div id="doc-cat-list">
          {snap.rows.map((row) => (
            <div key={row.id} className="doc-cat-row" data-doc-cat={row.id}>
              <div className="doc-cat-row-main">
                <input
                  type="text"
                  className="doc-cat-label"
                  data-doc-cat-label={row.id}
                  value={row.label}
                  spellCheck={false}
                  autoComplete="off"
                  aria-label={`Name for ${row.id}`}
                  disabled={snap.busy}
                  onChange={(e) => setCategoryLabelDraft(row.id, e.target.value)}
                  onBlur={() => {
                    void saveCategoryLabel(row.id);
                  }}
                />
                <code className="doc-cat-id" data-doc-cat-id={row.id}>
                  {row.id}
                </code>
              </div>
              <label className="doc-cat-exposed">
                <input
                  type="checkbox"
                  data-doc-cat-exposed={row.id}
                  checked={row.exposed}
                  disabled={snap.busy}
                  onChange={(e) => void toggleCategoryExposed(row.id, e.target.checked)}
                />
                Exposed
              </label>
              <button
                type="button"
                className="doc-cat-action doc-cat-del"
                data-doc-cat-remove={row.id}
                data-tip="Remove this category"
                aria-label={`Remove ${row.id}`}
                disabled={snap.busy}
                onClick={() => void removeDocCategory(row.id)}
              >
                Remove
              </button>
            </div>
          ))}
        </div>
        <div id="settings-result-document" className="settings-result">
          {snap.error}
        </div>
      </div>
      <div id="settings-tab-document-add" className="settings-tab-panel" data-tab="add">
        <p className="doc-cat-lead">
          Type a name. The id is generated from it and cannot change after it is added.
        </p>
        <div className="doc-cat-row" data-doc-cat="new">
          <div className="doc-cat-row-main">
            <input
              id="doc-cat-new-label"
              type="text"
              className="doc-cat-label"
              placeholder="Name"
              value={snap.newLabel}
              spellCheck={false}
              autoComplete="off"
              aria-label="Name"
              disabled={snap.busy}
              onChange={(e) => setNewCategoryLabel(e.target.value)}
            />
            <code
              id="doc-cat-new-id"
              className={`doc-cat-id${newId ? '' : ' is-empty'}`}
            >
              {newId || 'id is generated from the name'}
            </code>
          </div>
          <label className="doc-cat-exposed">
            <input
              id="doc-cat-new-exposed"
              type="checkbox"
              checked={snap.newExposed}
              disabled={snap.busy}
              onChange={(e) => setNewCategoryExposed(e.target.checked)}
            />
            Exposed
          </label>
          <button
            id="btn-doc-cat-add"
            type="button"
            className="doc-cat-action"
            disabled={snap.busy || !newId}
            onClick={() => void addDocCategory()}
          >
            Add
          </button>
        </div>
        <div id="settings-result-document-add" className="settings-result">
          {snap.error}
        </div>
      </div>
    </div>
  );
}
