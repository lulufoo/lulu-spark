import { NotesCategoriesAddHost } from './notes-categories-add.tsx';
import { NotesCategoriesEditHost } from './notes-categories-edit.tsx';

export function NotesSettingsPanel() {
  return (
    <div id="settings-panel-notes" className="settings-panel active">
      <div className="settings-tabs" role="tablist">
        <button type="button" className="settings-tab active" data-tab="directory" role="tab">
          Directory
        </button>
        <button type="button" className="settings-tab" data-tab="add" role="tab">
          Add category
        </button>
        <button type="button" className="settings-tab" data-tab="edit" role="tab">
          Edit &amp; Delete category
        </button>
      </div>
      <div
        id="settings-tab-notes-directory"
        className="settings-tab-panel active"
        data-tab="directory"
      >
        <div className="settings-field">
          <label htmlFor="notes-root-path">Notes directory</label>
          <input
            id="notes-root-path"
            type="text"
            spellCheck={false}
            autoComplete="off"
            readOnly
            className="settings-input-readonly"
          />
          <span className="settings-field-hint">
            Fixed notes root. Not configurable.
          </span>
        </div>
      </div>
      <div id="settings-tab-notes-add" className="settings-tab-panel" data-tab="add">
        <NotesCategoriesAddHost />
      </div>
      <div id="settings-tab-notes-edit" className="settings-tab-panel" data-tab="edit">
        <NotesCategoriesEditHost />
      </div>
    </div>
  );
}
