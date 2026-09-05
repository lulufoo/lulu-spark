import { useEffect, useSyncExternalStore } from 'react';
import { startIndexRebuild, watchIndexRebuildOnBoot } from '../commands/index-rebuild.ts';
import { indexRebuildStore } from '../state/index-rebuild.ts';

function titleFor(status: string, log: string) {
  if (status === 'running') return log ? `Rebuilding index: ${log}` : 'Rebuilding index…';
  if (status === 'error') return `Rebuild failed: ${log || 'Unknown error'}`;
  if (status === 'done') return `Rebuild search index (last run: ${log || 'ok'})`;
  return 'Rebuild search index';
}

/** Header-only entry for rebuilding the keyword index. Spins while running. */
export function IndexRebuildButton() {
  const state = useSyncExternalStore(indexRebuildStore.subscribe, indexRebuildStore.getSnapshot);
  const running = state.status === 'running';

  useEffect(() => {
    void watchIndexRebuildOnBoot();
  }, []);

  return (
    <button
      id="btn-index-rebuild"
      type="button"
      className={`gs-rebuild-btn header-index-btn${running ? ' syncing' : ''}${state.status === 'error' ? ' error' : ''}`}
      title={titleFor(state.status, state.log)}
      disabled={running}
      aria-busy={running}
      onClick={() => void startIndexRebuild()}
    >
      ↺ Index
    </button>
  );
}
