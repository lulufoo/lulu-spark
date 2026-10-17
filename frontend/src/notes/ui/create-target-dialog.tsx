import { useEffect, useMemo, useSyncExternalStore } from 'react';
import { OverlayDismissButton } from '../../shared/overlay-dismiss-button.tsx';
import { Select } from '../../shared/components/select/index.ts';
import { useHostState } from '../state/host.ts';
import { createTargetStore } from '../state/create-target.ts';
import { sortedProjectFolders } from '../state/dir-tree.ts';
import { cancelCreateProject, chooseCreateProject, confirmCreateProject } from '../commands/create-document.ts';

/** Asks which Project a new document goes to; shown only when none is selected (timeline All). */
export function CreateTargetDialog() {
  const host = useHostState();
  const snap = useSyncExternalStore(createTargetStore.subscribe, createTargetStore.getSnapshot);
  const titles = host.index.topicTitles;
  const folders = sortedProjectFolders(titles, host.index.topicOrder);
  const options = useMemo(
    () => folders.map((folder) => ({ value: folder, label: titles[folder] || folder })),
    [folders.join('\n'), titles],
  );

  useEffect(() => {
    if (!snap.open) return undefined;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      e.preventDefault();
      cancelCreateProject();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [snap.open]);

  if (!snap.open) return null;
  return (
    <div
      id="create-target-dialog"
      className="create-target-overlay"
      role="presentation"
      onClick={(e) => {
        if (e.target === e.currentTarget) cancelCreateProject();
      }}
    >
      <div className="create-target-box" role="dialog" aria-modal="true" aria-labelledby="create-target-title">
        <div className="create-target-header">
          <h3 id="create-target-title">New document</h3>
          <OverlayDismissButton label="Cancel" onClick={() => cancelCreateProject()} />
        </div>
        <div className="create-target-body">
          <label className="create-target-label">Create it in project</label>
          <Select ariaLabel="Project" value={snap.project} options={options} onChange={chooseCreateProject} />
          <div className="create-target-actions">
            <button type="button" className="btn-settings-ghost" onClick={() => cancelCreateProject()}>
              Cancel
            </button>
            <button
              id="btn-create-target-confirm"
              type="button"
              className="btn-settings-save"
              disabled={!snap.project}
              onClick={() => confirmCreateProject()}
            >
              Create
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
