import { useSyncExternalStore } from 'react';
import { addNotesCategory } from '../../commands/settings/notes-categories.ts';
import { notesCategoriesStore } from '../../state/settings/notes-categories.ts';

function field(id: string): HTMLInputElement | HTMLTextAreaElement | null {
  return document.getElementById(id) as HTMLInputElement | HTMLTextAreaElement | null;
}

function readAddForm() {
  return {
    title: field('notes-cat-new-title')?.value.trim() || '',
    description: field('notes-cat-new-description')?.value.trim() || '',
  };
}

function clearAddForm() {
  const title = field('notes-cat-new-title');
  const description = field('notes-cat-new-description');
  if (title) title.value = '';
  if (description) description.value = '';
}

async function onAdd() {
  const { title, description } = readAddForm();
  if (!title) return;
  await addNotesCategory(title, description);
  if (!notesCategoriesStore.getSnapshot().error) clearAddForm();
}

export function NotesCategoriesAddHost() {
  const snap = useSyncExternalStore(
    notesCategoriesStore.subscribe,
    notesCategoriesStore.getSnapshot,
  );

  return (
    <div className="notes-cat-add">
      <p className="notes-cat-lead">
        Adding a category does not move existing notes. Title and description are display only.
      </p>
      <div className="settings-field">
        <label htmlFor="notes-cat-new-title">Title</label>
        <input
          id="notes-cat-new-title"
          type="text"
          placeholder="Display title"
          autoComplete="off"
          spellCheck={false}
          disabled={snap.busy}
        />
      </div>
      <div className="settings-field">
        <label htmlFor="notes-cat-new-description">Description</label>
        <textarea
          id="notes-cat-new-description"
          rows={3}
          placeholder="Optional. Shown on the category list and note cards."
          autoComplete="off"
          spellCheck={false}
          disabled={snap.busy}
        />
      </div>
      <div id="notes-cat-add-error" className="settings-result notes-cat-error">
        {snap.error}
      </div>
      <div className="settings-panel-actions">
        <button
          id="btn-notes-cat-add"
          type="button"
          className="btn-settings-save"
          disabled={snap.busy}
          onClick={() => void onAdd()}
        >
          {snap.busy ? 'Adding…' : 'Add category'}
        </button>
      </div>
    </div>
  );
}
