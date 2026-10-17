import { useState, useSyncExternalStore } from 'react';
import { addNotesCategory } from '../../commands/settings/notes-categories.ts';
import { Icon } from '../../../shared/icons/icon.tsx';
import { DEFAULT_PROJECT_ICON_ID } from '../../../shared/icons/index.ts';
import { notesCategoriesStore } from '../../state/settings/notes-categories.ts';
import { IconPicker } from './icon-picker.tsx';

/** Add form for the selected workspace; it clears only after the host accepted the project. */
export function NotesCategoriesAddHost() {
  const snap = useSyncExternalStore(
    notesCategoriesStore.subscribe,
    notesCategoriesStore.getSnapshot,
  );
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [icon, setIcon] = useState('');

  async function onAdd() {
    const trimmed = title.trim();
    if (!trimmed) return;
    await addNotesCategory(snap.workspaceId, trimmed, description.trim(), icon);
    if (!notesCategoriesStore.getSnapshot().error) {
      setTitle('');
      setDescription('');
      setIcon('');
    }
  }

  return (
    <div className="notes-cat-add">
      <p className="notes-cat-lead">
        Adding a project does not move existing documents. Title and description are display only.
      </p>
      <div className="settings-field">
        <label htmlFor="notes-cat-new-title">Title</label>
        <div className="icon-name-row">
          <IconPicker
            label="Project icon"
            value={icon}
            defaultIcon={<Icon id={DEFAULT_PROJECT_ICON_ID} size={18} />}
            disabled={snap.busy}
            onChange={setIcon}
          />
          <input
            id="notes-cat-new-title"
            type="text"
            placeholder="Display title"
            autoComplete="off"
            spellCheck={false}
            disabled={snap.busy}
            value={title}
            onChange={(e) => setTitle(e.target.value)}
          />
        </div>
      </div>
      <div className="settings-field">
        <label htmlFor="notes-cat-new-description">Description</label>
        <textarea
          id="notes-cat-new-description"
          rows={3}
          placeholder="Optional. Shown on the project list."
          autoComplete="off"
          spellCheck={false}
          disabled={snap.busy}
          value={description}
          onChange={(e) => setDescription(e.target.value)}
        />
      </div>
      <div id="notes-cat-add-error" className="settings-result notes-cat-error">
        {snap.editorOpen ? '' : snap.error}
      </div>
      <div className="settings-panel-actions">
        <button
          id="btn-notes-cat-add"
          type="button"
          className="btn-settings-save"
          disabled={snap.busy || !title.trim()}
          onClick={() => void onAdd()}
        >
          {snap.busy ? 'Adding…' : 'Add project'}
        </button>
      </div>
    </div>
  );
}
