import { useEffect, useRef, useSyncExternalStore } from 'react';
import { loadBoardPatStatus, removeBoardPat, saveBoardPat } from '../../commands/settings/board.ts';
import { settingsOpenStore } from '../../state/dialog-open.ts';
import { boardPatStore } from '../../state/settings/board-pat.ts';
import { SAVED_KEY_PLACEHOLDER } from '../../state/settings/store.ts';

function hintFor(configured: boolean | null) {
  if (configured === true) return 'A Board token is saved. Enter a new token to replace it.';
  if (configured === false) return 'No Board token saved.';
  return '';
}

export function BoardSettingsPanel() {
  const snap = useSyncExternalStore(boardPatStore.subscribe, boardPatStore.getSnapshot);
  const open = useSyncExternalStore(settingsOpenStore.subscribe, settingsOpenStore.getSnapshot);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (open) void loadBoardPatStatus();
  }, [open]);

  useEffect(() => {
    const input = inputRef.current;
    if (!input) return;
    if (snap.configured === true && input.value === '') input.value = SAVED_KEY_PLACEHOLDER;
    if (snap.configured === false && input.value === SAVED_KEY_PLACEHOLDER) input.value = '';
  }, [snap.configured]);

  async function onSave(event: { preventDefault: () => void }) {
    event.preventDefault();
    const input = inputRef.current;
    const saved = await saveBoardPat(input?.value ?? '');
    if (saved && input) input.value = SAVED_KEY_PLACEHOLDER;
  }

  async function onRemove() {
    const removed = await removeBoardPat();
    if (removed && inputRef.current) inputRef.current.value = '';
  }

  function onFocus() {
    const input = inputRef.current;
    if (input?.value === SAVED_KEY_PLACEHOLDER) input.value = '';
  }

  function onBlur() {
    const input = inputRef.current;
    if (input && input.value === '' && boardPatStore.getSnapshot().configured === true) {
      input.value = SAVED_KEY_PLACEHOLDER;
    }
  }

  return (
    <div id="settings-panel-board" className="settings-panel">
      <form onSubmit={(event) => void onSave(event)}>
        <div className="settings-field">
          <label htmlFor="settings-board-pat">Board personal access token</label>
          <input
            ref={inputRef}
            id="settings-board-pat"
            type="password"
            autoComplete="off"
            spellCheck={false}
            aria-describedby="settings-board-pat-hint"
            onFocus={onFocus}
            onBlur={onBlur}
          />
          <span id="settings-board-pat-hint" className="settings-field-hint">
            {hintFor(snap.configured)}
          </span>
        </div>
        <div id="settings-result-board" className="settings-result" role="status">
          {snap.message}
        </div>
        <div className="settings-panel-actions">
          <button type="submit" className="btn-settings-save" disabled={snap.busy}>
            Save
          </button>
          <button type="button" className="btn-settings-ghost" disabled={snap.busy} onClick={() => void onRemove()}>
            Remove
          </button>
        </div>
      </form>
    </div>
  );
}
