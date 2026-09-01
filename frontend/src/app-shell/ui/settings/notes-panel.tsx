import { NotesCategoriesAddHost } from './notes-categories-add.tsx';
import { NotesCategoriesEditHost } from './notes-categories-edit.tsx';

export function NotesSettingsPanel() {
  return (
    <div id="settings-panel-notes" className="settings-panel">
      <div className="settings-tabs" role="tablist">
        <button type="button" className="settings-tab active" data-tab="add" role="tab">
          Add
        </button>
        <button type="button" className="settings-tab" data-tab="edit" role="tab">
          Edit &amp; Delete
        </button>
      </div>
      <div id="settings-tab-notes-add" className="settings-tab-panel active" data-tab="add">
        <NotesCategoriesAddHost />
      </div>
      <div id="settings-tab-notes-edit" className="settings-tab-panel" data-tab="edit">
        <NotesCategoriesEditHost />
      </div>
    </div>
  );
}
