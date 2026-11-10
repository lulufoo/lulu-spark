import { useEffect, useRef, useSyncExternalStore } from 'react';
import { loadBoardPatStatus, removeBoardPat, saveBoardPat } from '../../commands/settings/board.ts';
import { settingsOpenStore } from '../../state/dialog-open.ts';
import { boardPatStore } from '../../state/settings/board-pat.ts';

export function BoardSettingsPanel() {
  const snap = useSyncExternalStore(boardPatStore.subscribe, boardPatStore.getSnapshot);
  const open = useSyncExternalStore(settingsOpenStore.subscribe, settingsOpenStore.getSnapshot);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (open) void loadBoardPatStatus();
  }, [open]);

  async function onSave(event: { preventDefault: () => void }) {
    event.preventDefault();
    const input = inputRef.current;
    const saved = await saveBoardPat(input?.value ?? '');
    if (saved && input) input.value = '';
  }

  async function onRemove() {
    const removed = await removeBoardPat();
    if (removed && inputRef.current) inputRef.current.value = '';
  }

  return (
    <div id="settings-panel-board" className="settings-panel">
      <h3>Board</h3>
      <p className="settings-field-hint board-pat-lead">Connect your Board account to preview Sketch links.</p>
      {snap.configured === null ? null : (
        <p className={snap.configured ? 'board-pat-status is-set' : 'board-pat-status'}>
          {snap.configured ? 'Board PAT configured' : 'No Board token saved.'}
        </p>
      )}
      <form onSubmit={(event) => void onSave(event)}>
        <div className="settings-field">
          <label htmlFor="settings-board-pat">Board PAT</label>
          <input
            ref={inputRef}
            id="settings-board-pat"
            type="password"
            autoComplete="off"
            spellCheck={false}
            placeholder="Paste a new personal access token"
            aria-describedby="settings-board-pat-hint"
          />
          <span id="settings-board-pat-hint" className="settings-field-hint">
            Your saved token stays hidden and is stored securely.
          </span>
        </div>
        <div id="settings-result-board" className="settings-result" role="status">
          {snap.message}
        </div>
        <div className="settings-panel-actions">
          <button type="button" className="btn-settings-ghost" disabled={snap.busy} onClick={() => void onRemove()}>
            Remove
          </button>
          <button type="submit" className="btn-settings-save" disabled={snap.busy}>
            Save
          </button>
        </div>
      </form>
    </div>
  );
}
