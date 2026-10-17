import { useState, useSyncExternalStore } from 'react';
import { addNotesCategory } from '../../commands/settings/notes-categories.ts';
import { notesCategoriesStore } from '../../state/settings/notes-categories.ts';

/** Add form for the selected workspace; it clears only after the host accepted the project. */
export function WorkspaceProjectAdd() {
  const snap = useSyncExternalStore(
    notesCategoriesStore.subscribe,
    notesCategoriesStore.getSnapshot,
  );
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');

  async function onAdd() {
    const trimmed = title.trim();
    if (!trimmed) return;
    await addNotesCategory(snap.workspaceId, trimmed, description.trim());
    if (!notesCategoriesStore.getSnapshot().error) {
      setTitle('');
      setDescription('');
    }
  }

  return (
    <div className="notes-cat-add">
      <div className="settings-field">
        <label htmlFor="notes-cat-new-title">New project</label>
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
      <div className="settings-field">
        <label htmlFor="notes-cat-new-description">Description</label>
        <textarea
          id="notes-cat-new-description"
          rows={3}
          placeholder="Optional. Shown on the project list and note cards."
          autoComplete="off"
          spellCheck={false}
          disabled={snap.busy}
          value={description}
          onChange={(e) => setDescription(e.target.value)}
        />
      </div>
      <div className="settings-panel-actions">
        <button
          id="btn-notes-cat-add"
          type="button"
          className="btn-settings-save"
          disabled={snap.busy || !title.trim()}
          onClick={() => void onAdd()}
        >
          {snap.busy ? 'Working…' : 'Add project'}
        </button>
      </div>
    </div>
  );
}
