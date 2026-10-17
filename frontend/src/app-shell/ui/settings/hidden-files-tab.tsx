import { useSyncExternalStore } from 'react';
import {
  addHidePatternRow,
  cancelHidePatternEdit,
  removeHidePatternRow,
  saveHidePatternEdit,
  setHidePatternAddValue,
  setHidePatternDraft,
  startHidePatternEdit,
} from '../../commands/settings/hide-patterns.ts';
import { hidePatternsStore } from '../../state/settings/hide-patterns.ts';

export function HiddenFilesTab() {
  const snap = useSyncExternalStore(
    hidePatternsStore.subscribe,
    hidePatternsStore.getSnapshot,
  );

  return (
    <div id="settings-tab-workspace-hidden" className="settings-tab-panel" data-tab="hidden">
      <p className="settings-field-hint">
        Each row is one filename regex. A file or folder name matching any row is hidden in this
        workspace's tree, counts, and search collect. Example: <code>{'\\.xxx$'}</code>
      </p>
      <div id="settings-kb-hide-list">
        {snap.rows.map((row) => (
          <div key={row.id} className="kb-hide-row">
            {snap.editingId === row.id ? (
              <input
                className="kb-hide-row-input"
                type="text"
                value={snap.draft}
                spellCheck={false}
                autoComplete="off"
                disabled={snap.busy}
                autoFocus
                onChange={(e) => setHidePatternDraft(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    void saveHidePatternEdit();
                  }
                  if (e.key === 'Escape') {
                    e.preventDefault();
                    cancelHidePatternEdit();
                  }
                }}
                onBlur={() => {
                  void saveHidePatternEdit();
                }}
              />
            ) : (
              <button
                type="button"
                className="kb-hide-row-text"
                disabled={snap.busy}
                onClick={() => startHidePatternEdit(row.id)}
              >
                {row.pattern}
              </button>
            )}
            <button
              type="button"
              className="kb-hide-row-del"
              data-tip="Remove this rule"
              aria-label="Remove this rule"
              disabled={snap.busy}
              onClick={() => void removeHidePatternRow(row.id)}
            >
              ×
            </button>
          </div>
        ))}
      </div>
      <div className="settings-connect-row">
        <input
          id="settings-kb-hide-add"
          type="text"
          placeholder="Add regex"
          value={snap.addValue}
          spellCheck={false}
          autoComplete="off"
          disabled={snap.busy}
          onChange={(e) => setHidePatternAddValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault();
              void addHidePatternRow();
            }
          }}
        />
        <button
          id="btn-settings-kb-hide-add"
          type="button"
          className="btn-settings-save"
          disabled={snap.busy}
          onClick={() => void addHidePatternRow()}
        >
          Add
        </button>
      </div>
      <div id="settings-result-workspace-hidden" className="settings-result">
        {snap.error}
      </div>
    </div>
  );
}
