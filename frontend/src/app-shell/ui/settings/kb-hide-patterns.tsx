import { useSyncExternalStore } from 'react';
import {
  addKbHidePatternRow,
  cancelKbHidePatternEdit,
  removeKbHidePatternRow,
  saveKbHidePatternEdit,
  setKbHidePatternAddValue,
  setKbHidePatternDraft,
  startKbHidePatternEdit,
} from '../../commands/settings/kb-hide-patterns.ts';
import { kbHidePatternsStore } from '../../state/settings/kb-hide-patterns.ts';

export function KbHidePatternsHost() {
  const snap = useSyncExternalStore(
    kbHidePatternsStore.subscribe,
    kbHidePatternsStore.getSnapshot,
  );

  return (
    <div id="settings-tab-knowledge-hidden" className="settings-tab-panel" data-tab="hidden">
      <p className="settings-field-hint">
        Each row is one filename regex. A name matching any row is hidden in the tree, counts, and
        search collect. Example: <code>{'\\.xxx$'}</code>
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
                onChange={(e) => setKbHidePatternDraft(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    void saveKbHidePatternEdit();
                  }
                  if (e.key === 'Escape') {
                    e.preventDefault();
                    cancelKbHidePatternEdit();
                  }
                }}
                onBlur={() => {
                  void saveKbHidePatternEdit();
                }}
              />
            ) : (
              <button
                type="button"
                className="kb-hide-row-text"
                disabled={snap.busy}
                onClick={() => startKbHidePatternEdit(row.id)}
              >
                {row.pattern}
              </button>
            )}
            <button
              type="button"
              className="kb-hide-row-del"
              title="Remove this rule"
              disabled={snap.busy}
              onClick={() => void removeKbHidePatternRow(row.id)}
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
          onChange={(e) => setKbHidePatternAddValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault();
              void addKbHidePatternRow();
            }
          }}
        />
        <button
          id="btn-settings-kb-hide-add"
          type="button"
          className="btn-settings-save"
          disabled={snap.busy}
          onClick={() => void addKbHidePatternRow()}
        >
          Add
        </button>
      </div>
      <div id="settings-result-knowledge" className="settings-result">
        {snap.error}
      </div>
    </div>
  );
}
